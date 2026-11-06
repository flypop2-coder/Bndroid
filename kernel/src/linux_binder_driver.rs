//! Linux syscall adapter for the Binder router. All calls run with IRQ masked.
//! Payloads are copied into kernel storage before publication. Completion user
//! copies happen only after the receiver's TTBR and kernel stack are installed.
use crate::{linux_runtime as runtime, process};
use alloc::vec::Vec;
use bndroid_kernel::{
    linux_binder::{self as binder, Delivery, Mapping, Parcel, wire::*},
    linux_fd::FileRef,
};

pub struct ReadCall {
    pub file: FileRef,
    pub address: u64,
    pub io: WriteRead,
    pub tid: u32,
}
pub enum Call {
    Complete(u64),
    Wait(ReadCall),
}

fn output(address: u64, bytes: &[u8]) -> Result<(), i64> {
    let result = runtime::copy_out(address, bytes) as i64;
    if result < 0 { Err(-result) } else { Ok(()) }
}
fn authenticate(file: &FileRef) -> Result<u64, i64> {
    let device = file.binder().ok_or(9)?;
    binder::with(|r| r.authenticate(device.id, process::current_live_user_process_id().ok_or(3)?))?;
    Ok(device.id)
}
pub fn map(args: [u64; 6], file: &FileRef) -> Result<u64, i64> {
    let id = authenticate(file)?;
    binder::with(|r| r.can_map(id))?;
    let length = bndroid_kernel::linux_vm::rounded_length(args[1])?;
    if length > MAX_MAPPING {
        return Err(12);
    }
    let backing = &file.binder().unwrap().backing;
    let memory = backing.memory().unwrap();
    process::linux_memfd_resize(memory, length)?;
    let bndroid_kernel::linux_fd::Target::Binder(domain) = file.target() else {
        return Err(9);
    };
    let result = process::with_current_linux_address_space(|space| {
        space.linux_mmap_binder(args, backing, domain.path())
    });
    match result {
        Ok(base) => {
            binder::with(|r| {
                r.map(
                    id,
                    Mapping {
                        base,
                        length,
                        inode: memory.inode(),
                    },
                )
            })?;
            Ok(base)
        }
        Err(error) => {
            process::linux_memfd_resize(memory, 0)?;
            Err(error)
        }
    }
}
pub fn ioctl(r: &[u64; 31], tid: u32) -> Option<Result<Call, i64>> {
    if r[8] != 29 {
        return None;
    }
    let file = match process::with_linux_files(|t| t.file(r[0])) {
        Ok(file) if file.binder().is_some() => file,
        _ => return None,
    };
    Some(start(file, tid, r[1], r[2]))
}
#[inline(never)]
fn start(file: FileRef, tid: u32, request: u64, address: u64) -> Result<Call, i64> {
    let id = authenticate(&file)?;
    match request {
        VERSION => output(address, &8_u32.to_le_bytes())?,
        SET_MAX_THREADS => {
            let mut b = [0; 4];
            runtime::copy_in(address, &mut b)?;
            binder::with(|r| r.set_max_threads(id, u32::from_le_bytes(b)))?;
        }
        SET_CONTEXT_MGR => binder::with(|r| r.set_manager(id, 0, 0))?,
        SET_CONTEXT_MGR_EXT => {
            let mut b = [0; 24];
            runtime::copy_in(address, &mut b)?;
            // Security contexts and RT inheritance need separate policy.
            if u32_at(&b, 4) & !0x1ff != 0 {
                return Err(95);
            }
            binder::with(|r| {
                r.set_manager_with_flags(id, u64_at(&b, 8), u64_at(&b, 16), u32_at(&b, 4))
            })?;
        }
        THREAD_EXIT => binder::with(|r| r.thread_exit(id, tid)),
        WRITE_READ => return write_read(file, tid, address),
        _ => return Err(22),
    }
    Ok(Call::Complete(0))
}
#[inline(never)]
fn write_read(file: FileRef, tid: u32, address: u64) -> Result<Call, i64> {
    let mut bytes = [0; 48];
    runtime::copy_in(address, &mut bytes)?;
    let mut io = WriteRead::decode(&bytes)?;
    if !process::current_user_range_is_writable(runtime::user_address(address), 48) {
        return Err(14);
    }
    let id = authenticate(&file)?;
    for _ in 0..256 {
        if io.write_consumed == io.write_size {
            break;
        }
        let result = (|| {
            if io.write_size - io.write_consumed < 4 {
                return Err(22);
            }
            let mut opcode = [0; 4];
            runtime::copy_in(io.write_buffer + io.write_consumed, &mut opcode)?;
            let command = u32::from_le_bytes(opcode);
            let size = command_bytes(command)?;
            if io.write_size - io.write_consumed - 4 < size as u64 {
                return Err(22);
            }
            let mut body = [0; 64];
            runtime::copy_in(io.write_buffer + io.write_consumed + 4, &mut body[..size])?;
            execute(&file, tid, command, &body)?;
            io.write_consumed += 4 + size as u64;
            Ok(())
        })();
        if let Err(error) = result {
            io.read_consumed = 0;
            output(address, &io.encode())?;
            return Err(error);
        }
        if binder::with(|r| r.has_error(id, tid)) {
            break;
        }
    }
    output(address, &io.encode())?;
    if io.read_size == 0 {
        return Ok(Call::Complete(0));
    }
    let call = ReadCall {
        file,
        address,
        io,
        tid,
    };
    resume(call)
}
pub fn resume(call: ReadCall) -> Result<Call, i64> {
    while let Some(delivery) = poll(&call)? {
        if finish(&call, delivery)? {
            return Ok(Call::Complete(0));
        }
    }
    if call.file.flags() & 0x800 != 0 {
        cancel(&call, None);
        return Err(11);
    }
    binder::with(|r| r.waiting(call.file.binder().unwrap().id, call.tid, true))?;
    Ok(Call::Wait(call))
}
#[inline(never)]
fn execute(file: &FileRef, tid: u32, command: u32, body: &[u8; 64]) -> Result<(), i64> {
    let id = file.binder().unwrap().id;
    match command {
        BC_TRANSACTION | BC_REPLY => {
            let data = Transaction::decode(body);
            let payload = (|| {
                data.validate_payload()?;
                let mut bytes = Vec::new();
                let length = data.payload_length()?;
                bytes.try_reserve_exact(length).map_err(|_| 12)?;
                bytes.resize(length, 0);
                runtime::copy_in(data.data, &mut bytes[..data.data_size as usize])?;
                if data.offsets_size != 0 {
                    let at = (data.data_size as usize + 7) & !7;
                    runtime::copy_in(data.offsets, &mut bytes[at..])?;
                }
                process::with_linux_files(|table| Parcel::capture(data, bytes, table))
            })();
            let result = match payload {
                Ok(bytes) => binder::with(|r| {
                    if command == BC_TRANSACTION {
                        r.transact_parcel(id, tid, data, bytes)
                    } else {
                        r.reply_parcel(id, tid, data, bytes)
                    }
                }),
                Err(error) => Err(error),
            };
            let prepared_error = binder::with(|r| r.stats().last_transfer_error);
            if result.is_ok() && prepared_error != 0 {
                crate::kprintln!(
                    "LINUX_BINDER_PREPARE_FAILED tid={} operation={} code={} data_bytes={} offsets_bytes={} errno={}",
                    tid,
                    command,
                    data.code,
                    data.data_size,
                    data.offsets_size,
                    prepared_error
                );
            }
            if let Err(error) = result {
                crate::kprintln!(
                    "LINUX_BINDER_TRANSFER_FAILED tid={} operation={} code={} data_bytes={} offsets_bytes={} errno={}",
                    tid,
                    command,
                    data.code,
                    data.data_size,
                    data.offsets_size,
                    error
                );
                binder::with(|r| {
                    if command == BC_REPLY {
                        r.fail_reply(id, tid)
                    } else {
                        r.fail_transaction(id, tid)
                    }
                })?;
            }
        }
        BC_FREE_BUFFER => {
            let (offset, length) = binder::with(|r| r.free_buffer(id, u64_at(body, 0)))?;
            clear_pool(file, offset, length)?;
        }
        BC_ENTER_LOOPER | BC_REGISTER_LOOPER | BC_EXIT_LOOPER => {
            binder::with(|r| r.looper(id, tid, command))?
        }
        BC_INCREFS | BC_ACQUIRE | BC_RELEASE | BC_DECREFS => {
            binder::with(|r| r.reference(id, u32_at(body, 0), command))?
        }
        BC_INCREFS_DONE | BC_ACQUIRE_DONE => {
            binder::with(|r| r.acknowledge(id, u64_at(body, 0), u64_at(body, 8), command))?
        }
        BC_REQUEST_DEATH => {
            binder::with(|r| r.request_death(id, u32_at(body, 0), u64_at(body, 4)))?
        }
        BC_CLEAR_DEATH => binder::with(|r| r.clear_death(id, u32_at(body, 0), u64_at(body, 4)))?,
        BC_DEAD_DONE => binder::with(|r| r.dead_done(id, u64_at(body, 0)))?,
        _ => return Err(22),
    }
    Ok(())
}
#[inline(never)]
fn clear_pool(file: &FileRef, offset: u64, length: u64) -> Result<(), i64> {
    let zero = [0; 512];
    let memory = file.binder().unwrap().backing.memory().unwrap();
    let mut at = 0;
    while at < length {
        let count = (length - at).min(zero.len() as u64);
        memory.write(offset + at, &zero[..count as usize])?;
        at += count;
    }
    Ok(())
}
pub fn poll(call: &ReadCall) -> Result<Option<Delivery>, i64> {
    let id = call.file.binder().unwrap().id;
    let available = call.io.read_size - call.io.read_consumed;
    let capacity = available
        .checked_add(if call.io.read_consumed != 0 { 4 } else { 0 })
        .ok_or(22)?;
    binder::with(|r| {
        r.waiting(id, call.tid, false)?;
        let result = r.read(id, call.tid, capacity as usize);
        if matches!(result, Ok(None)) {
            r.waiting(id, call.tid, true)?;
        }
        result
    })
}
#[inline(never)]
pub fn finish(call: &ReadCall, mut delivery: Delivery) -> Result<bool, i64> {
    let id = call.file.binder().unwrap().id;
    let mut received = Vec::new();
    let result = (|| {
        binder::with(|r| r.validate_delivery(id, &delivery))?;
        let skip = if call.io.read_consumed == 0 { 0 } else { 4 };
        // Validate both destinations again: another thread may have unmapped
        // them while this ioctl slept. No stale user pointer is trusted.
        if !process::current_user_range_is_writable(runtime::user_address(call.address), 48)
            || !process::current_user_range_is_writable(
                runtime::user_address(call.io.read_buffer + call.io.read_consumed),
                delivery.length - skip,
            )
        {
            return Err(14);
        }
        if !delivery.files.is_empty() {
            match process::with_linux_files(|table| table.receive_files(&delivery.files)) {
                Ok(numbers) => received = numbers,
                Err(_) => {
                    let reply = delivery.is_reply();
                    binder::with(|r| r.reject_files(id, call.tid, &delivery));
                    if !reply {
                        return Ok(false);
                    }
                    delivery = Delivery::failed_reply();
                }
            }
            if let Some(payload) = delivery.payload.as_mut() {
                for (transfer, number) in delivery.files.iter().zip(&received) {
                    put32(payload, transfer.offset(), *number);
                }
            }
        }
        if let (Some(offset), Some(payload)) = (delivery.offset, delivery.payload.as_ref()) {
            call.file
                .binder()
                .unwrap()
                .backing
                .memory()
                .unwrap()
                .write(offset, payload)?;
        }
        let bytes = &delivery.bytes[skip..delivery.length];
        output(call.io.read_buffer + call.io.read_consumed, bytes)?;
        let mut io = call.io;
        io.read_consumed += bytes.len() as u64;
        output(call.address, &io.encode())?;
        binder::with(|r| r.finish_delivery(id, &delivery))?;
        Ok(true)
    })();
    if let Err(error) = result {
        process::with_linux_files(|table| table.undo_received_files(&received, &delivery.files));
        crate::kprintln!(
            "LINUX_BINDER_DELIVERY_FAILED tid={} bytes={} errno={}",
            call.tid,
            delivery.length,
            error
        );
        // A failed user copy must also honor CLEAR_BUF. The recipient owns
        // this reserved slot until discard; other buffers cannot reuse it.
        if binder::with(|r| r.validate_delivery(id, &delivery)).is_ok()
            && let (Some(offset), Some(payload)) = (delivery.offset, delivery.payload.as_ref())
        {
            let length = (payload.len().max(1) as u64 + 7) & !7;
            let _ = clear_pool(&call.file, offset, length);
        }
        binder::with(|r| r.discard_delivery(id, &delivery));
    }
    result
}
pub fn cancel(call: &ReadCall, ready: Option<Delivery>) {
    let id = call.file.binder().unwrap().id;
    binder::with(|r| {
        let _ = r.waiting(id, call.tid, false);
        if let Some(delivery) = ready {
            r.discard_delivery(id, &delivery);
        }
    });
}
