//! Allocation-free publisher scene model for the opt-in AndroidBox
//! `SceneRPC-2` profile.
//!
//! This is deliberately a UI-side value model, not an Android View
//! implementation. The compatibility worker has already verified the APK,
//! DEX callback and compiled resources before these bounded nodes arrive.

pub const ANDROID_ACTIVITY_SCENE_MAX_NODES: usize = 8;
pub const ANDROID_ACTIVITY_SCENE_MAX_CALLBACK_BUTTONS: usize = 4;
pub const ANDROID_ACTIVITY_SCENE_TEXT_CAPACITY: usize = 96;
pub const ANDROID_ACTIVITY_BUTTON_TEXT_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AndroidSceneViewKind {
    LinearLayout = 1,
    TextView = 2,
    Button = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AndroidSceneLayoutSize {
    MatchParent = 1,
    WrapContent = 2,
    #[cfg(feature = "androidbox-layout-weight15")]
    Zero = 3,
    #[cfg(feature = "androidbox-layout-size18")]
    Exact = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AndroidSceneOrientation {
    None = 0,
    Vertical = 1,
    #[cfg(feature = "androidbox-layout-row14")]
    Horizontal = 2,
}

impl AndroidSceneOrientation {
    const fn is_linear_layout(self) -> bool {
        match self {
            Self::Vertical => true,
            #[cfg(feature = "androidbox-layout-row14")]
            Self::Horizontal => true,
            Self::None => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidSceneText {
    bytes: [u8; ANDROID_ACTIVITY_SCENE_TEXT_CAPACITY],
    len: u8,
}

impl AndroidSceneText {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; ANDROID_ACTIVITY_SCENE_TEXT_CAPACITY],
            len: 0,
        }
    }

    pub fn from_ascii(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > ANDROID_ACTIVITY_SCENE_TEXT_CAPACITY
            || !bytes.iter().all(|byte| matches!(*byte, 0x20..=0x7e))
        {
            return None;
        }
        let mut result = Self::empty();
        result.bytes[..bytes.len()].copy_from_slice(bytes);
        result.len = u8::try_from(bytes.len()).ok()?;
        Some(result)
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub fn as_str(&self) -> &str {
        // Admission accepts only printable ASCII, which is valid UTF-8.
        core::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("Android scene ASCII invariant")
    }
}

impl Default for AndroidSceneText {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledActivitySceneNode {
    kind: AndroidSceneViewKind,
    parent: Option<u8>,
    id: u32,
    width: AndroidSceneLayoutSize,
    height: AndroidSceneLayoutSize,
    orientation: AndroidSceneOrientation,
    layout_weight: u8,
    layout_margin_dp: u8,
    padding_dp: u8,
    layout_margin_left_dp: u8,
    layout_margin_top_dp: u8,
    layout_margin_right_dp: u8,
    layout_margin_bottom_dp: u8,
    padding_left_dp: u8,
    padding_top_dp: u8,
    padding_right_dp: u8,
    padding_bottom_dp: u8,
    exact_width_dp: u8,
    exact_height_dp: u8,
    text: AndroidSceneText,
    callback_registered: bool,
}

impl AndroidInstalledActivitySceneNode {
    pub const fn empty() -> Self {
        Self {
            kind: AndroidSceneViewKind::LinearLayout,
            parent: None,
            id: 0,
            width: AndroidSceneLayoutSize::WrapContent,
            height: AndroidSceneLayoutSize::WrapContent,
            orientation: AndroidSceneOrientation::None,
            layout_weight: 0,
            layout_margin_dp: 0,
            padding_dp: 0,
            layout_margin_left_dp: 0,
            layout_margin_top_dp: 0,
            layout_margin_right_dp: 0,
            layout_margin_bottom_dp: 0,
            padding_left_dp: 0,
            padding_top_dp: 0,
            padding_right_dp: 0,
            padding_bottom_dp: 0,
            exact_width_dp: 0,
            exact_height_dp: 0,
            text: AndroidSceneText::empty(),
            callback_registered: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
    ) -> Option<Self> {
        Self::try_new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            0,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-weight15")]
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_weighted(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
    ) -> Option<Self> {
        Self::try_new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            layout_weight,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-spacing16")]
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_spaced(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
        layout_margin_dp: u8,
        padding_dp: u8,
    ) -> Option<Self> {
        Self::try_new_with_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            layout_weight,
            layout_margin_dp,
            padding_dp,
        )
    }

    #[cfg(feature = "androidbox-layout-directional17")]
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_directional(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
    ) -> Option<Self> {
        Self::try_new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            layout_weight,
            0,
            0,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            0,
            0,
        )
    }

    #[cfg(feature = "androidbox-layout-size18")]
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_sized(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
        exact_width_dp: u8,
        exact_height_dp: u8,
    ) -> Option<Self> {
        Self::try_new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            layout_weight,
            0,
            0,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            exact_width_dp,
            exact_height_dp,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_new_with_layout(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
        layout_margin_dp: u8,
        padding_dp: u8,
    ) -> Option<Self> {
        Self::try_new_with_directional_layout(
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            text,
            callback_registered,
            layout_weight,
            layout_margin_dp,
            padding_dp,
            layout_margin_dp,
            layout_margin_dp,
            layout_margin_dp,
            layout_margin_dp,
            padding_dp,
            padding_dp,
            padding_dp,
            padding_dp,
            0,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_new_with_directional_layout(
        kind: AndroidSceneViewKind,
        parent: Option<u8>,
        id: u32,
        width: AndroidSceneLayoutSize,
        height: AndroidSceneLayoutSize,
        orientation: AndroidSceneOrientation,
        text: &str,
        callback_registered: bool,
        layout_weight: u8,
        layout_margin_dp: u8,
        padding_dp: u8,
        layout_margin_left_dp: u8,
        layout_margin_top_dp: u8,
        layout_margin_right_dp: u8,
        layout_margin_bottom_dp: u8,
        padding_left_dp: u8,
        padding_top_dp: u8,
        padding_right_dp: u8,
        padding_bottom_dp: u8,
        exact_width_dp: u8,
        exact_height_dp: u8,
    ) -> Option<Self> {
        #[cfg(feature = "androidbox-layout-weight15")]
        let weight_is_canonical = match layout_weight {
            0 => width != AndroidSceneLayoutSize::Zero && height != AndroidSceneLayoutSize::Zero,
            1..=8 => {
                kind == AndroidSceneViewKind::Button && width == AndroidSceneLayoutSize::Zero && {
                    #[cfg(feature = "androidbox-layout-size18")]
                    {
                        matches!(
                            height,
                            AndroidSceneLayoutSize::WrapContent | AndroidSceneLayoutSize::Exact
                        )
                    }
                    #[cfg(not(feature = "androidbox-layout-size18"))]
                    {
                        height == AndroidSceneLayoutSize::WrapContent
                    }
                }
            }
            _ => false,
        };
        #[cfg(not(feature = "androidbox-layout-weight15"))]
        let weight_is_canonical = layout_weight == 0;
        if !weight_is_canonical {
            return None;
        }
        #[cfg(feature = "androidbox-layout-spacing16")]
        let spacing_is_canonical = layout_margin_dp <= 16
            && padding_dp <= 16
            && [
                layout_margin_left_dp,
                layout_margin_top_dp,
                layout_margin_right_dp,
                layout_margin_bottom_dp,
                padding_left_dp,
                padding_top_dp,
                padding_right_dp,
                padding_bottom_dp,
            ]
            .iter()
            .all(|value| *value <= 16)
            && ((layout_margin_left_dp
                | layout_margin_top_dp
                | layout_margin_right_dp
                | layout_margin_bottom_dp)
                == 0
                || kind == AndroidSceneViewKind::Button)
            && ((padding_left_dp | padding_top_dp | padding_right_dp | padding_bottom_dp) == 0
                || kind == AndroidSceneViewKind::LinearLayout)
            && {
                #[cfg(feature = "androidbox-layout-directional17")]
                {
                    true
                }
                #[cfg(not(feature = "androidbox-layout-directional17"))]
                {
                    layout_margin_left_dp == layout_margin_dp
                        && layout_margin_top_dp == layout_margin_dp
                        && layout_margin_right_dp == layout_margin_dp
                        && layout_margin_bottom_dp == layout_margin_dp
                        && padding_left_dp == padding_dp
                        && padding_top_dp == padding_dp
                        && padding_right_dp == padding_dp
                        && padding_bottom_dp == padding_dp
                }
            };
        #[cfg(not(feature = "androidbox-layout-spacing16"))]
        let spacing_is_canonical = layout_margin_dp == 0 && padding_dp == 0;
        if !spacing_is_canonical {
            return None;
        }
        #[cfg(feature = "androidbox-layout-size18")]
        let size_is_canonical = (width == AndroidSceneLayoutSize::Exact) == (exact_width_dp != 0)
            && (height == AndroidSceneLayoutSize::Exact) == (exact_height_dp != 0);
        #[cfg(not(feature = "androidbox-layout-size18"))]
        let size_is_canonical = exact_width_dp == 0 && exact_height_dp == 0;
        if !size_is_canonical {
            return None;
        }
        let text = match kind {
            AndroidSceneViewKind::LinearLayout => {
                if !text.is_empty() || !orientation.is_linear_layout() || callback_registered {
                    return None;
                }
                AndroidSceneText::empty()
            }
            AndroidSceneViewKind::TextView => {
                if id == 0 || orientation != AndroidSceneOrientation::None || callback_registered {
                    return None;
                }
                AndroidSceneText::from_ascii(text)?
            }
            AndroidSceneViewKind::Button => {
                if id == 0
                    || orientation != AndroidSceneOrientation::None
                    || text.len() > ANDROID_ACTIVITY_BUTTON_TEXT_CAPACITY
                {
                    return None;
                }
                AndroidSceneText::from_ascii(text)?
            }
        };
        Some(Self {
            kind,
            parent,
            id,
            width,
            height,
            orientation,
            layout_weight,
            layout_margin_dp,
            padding_dp,
            layout_margin_left_dp,
            layout_margin_top_dp,
            layout_margin_right_dp,
            layout_margin_bottom_dp,
            padding_left_dp,
            padding_top_dp,
            padding_right_dp,
            padding_bottom_dp,
            exact_width_dp,
            exact_height_dp,
            text,
            callback_registered,
        })
    }

    pub const fn kind(self) -> AndroidSceneViewKind {
        self.kind
    }

    pub const fn parent(self) -> Option<u8> {
        self.parent
    }

    pub const fn id(self) -> u32 {
        self.id
    }

    pub const fn width(self) -> AndroidSceneLayoutSize {
        self.width
    }

    pub const fn height(self) -> AndroidSceneLayoutSize {
        self.height
    }

    pub const fn orientation(self) -> AndroidSceneOrientation {
        self.orientation
    }

    pub const fn layout_weight(self) -> u8 {
        self.layout_weight
    }

    pub const fn layout_margin_dp(self) -> u8 {
        self.layout_margin_dp
    }

    pub const fn padding_dp(self) -> u8 {
        self.padding_dp
    }

    pub const fn layout_margin_left_dp(self) -> u8 {
        self.layout_margin_left_dp
    }

    pub const fn layout_margin_top_dp(self) -> u8 {
        self.layout_margin_top_dp
    }

    pub const fn layout_margin_right_dp(self) -> u8 {
        self.layout_margin_right_dp
    }

    pub const fn layout_margin_bottom_dp(self) -> u8 {
        self.layout_margin_bottom_dp
    }

    pub const fn padding_left_dp(self) -> u8 {
        self.padding_left_dp
    }

    pub const fn padding_top_dp(self) -> u8 {
        self.padding_top_dp
    }

    pub const fn padding_right_dp(self) -> u8 {
        self.padding_right_dp
    }

    pub const fn padding_bottom_dp(self) -> u8 {
        self.padding_bottom_dp
    }

    pub const fn exact_width_dp(self) -> u8 {
        self.exact_width_dp
    }

    pub const fn exact_height_dp(self) -> u8 {
        self.exact_height_dp
    }

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub const fn callback_registered(self) -> bool {
        self.callback_registered
    }
}

impl Default for AndroidInstalledActivitySceneNode {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidInstalledActivitySceneState {
    nodes: [AndroidInstalledActivitySceneNode; ANDROID_ACTIVITY_SCENE_MAX_NODES],
    len: u8,
    revision: u64,
}

impl AndroidInstalledActivitySceneState {
    pub const fn empty() -> Self {
        Self {
            nodes: [AndroidInstalledActivitySceneNode::empty(); ANDROID_ACTIVITY_SCENE_MAX_NODES],
            len: 0,
            revision: 0,
        }
    }

    pub fn try_new(source: &[AndroidInstalledActivitySceneNode], revision: u64) -> Option<Self> {
        if source.is_empty() || source.len() > ANDROID_ACTIVITY_SCENE_MAX_NODES || revision == 0 {
            return None;
        }
        let mut result = Self::empty();
        let mut text_views = 0u8;
        let mut callback_buttons = 0u8;
        for (index, node) in source.iter().copied().enumerate() {
            let index_u8 = u8::try_from(index).ok()?;
            if index == 0 {
                if node.kind != AndroidSceneViewKind::LinearLayout
                    || node.parent.is_some()
                    || node.orientation != AndroidSceneOrientation::Vertical
                {
                    return None;
                }
            } else {
                let parent = node.parent?;
                if parent >= index_u8
                    || source[usize::from(parent)].kind != AndroidSceneViewKind::LinearLayout
                {
                    return None;
                }
                #[cfg(feature = "androidbox-layout-weight15")]
                if node.layout_weight != 0
                    && source[usize::from(parent)].orientation
                        != AndroidSceneOrientation::Horizontal
                {
                    return None;
                }
                #[cfg(feature = "androidbox-layout-mixed19")]
                if source[usize::from(parent)].orientation == AndroidSceneOrientation::Horizontal {
                    let fixed = node.kind == AndroidSceneViewKind::Button
                        && node.layout_weight == 0
                        && node.width == AndroidSceneLayoutSize::Exact;
                    let weighted = node.kind == AndroidSceneViewKind::Button
                        && node.layout_weight != 0
                        && node.width == AndroidSceneLayoutSize::Zero;
                    if !fixed && !weighted {
                        return None;
                    }
                }
            }
            if node.id != 0 && source[..index].iter().any(|earlier| earlier.id == node.id) {
                return None;
            }
            match node.kind {
                AndroidSceneViewKind::LinearLayout => {
                    if !node.text.is_empty()
                        || !node.orientation.is_linear_layout()
                        || node.callback_registered
                    {
                        return None;
                    }
                }
                AndroidSceneViewKind::TextView => {
                    if node.id == 0
                        || node.text.is_empty()
                        || node.orientation != AndroidSceneOrientation::None
                        || node.callback_registered
                    {
                        return None;
                    }
                    text_views = text_views.checked_add(1)?;
                }
                AndroidSceneViewKind::Button => {
                    if node.id == 0
                        || node.text.is_empty()
                        || node.text.as_str().len() > ANDROID_ACTIVITY_BUTTON_TEXT_CAPACITY
                        || node.orientation != AndroidSceneOrientation::None
                    {
                        return None;
                    }
                    if node.callback_registered {
                        callback_buttons = callback_buttons.checked_add(1)?;
                    }
                }
            }
            result.nodes[index] = node;
        }
        #[cfg(feature = "androidbox-multiaction3")]
        let callback_cardinality_valid =
            (1..=ANDROID_ACTIVITY_SCENE_MAX_CALLBACK_BUTTONS as u8).contains(&callback_buttons);
        #[cfg(not(feature = "androidbox-multiaction3"))]
        let callback_cardinality_valid = callback_buttons == 1;
        if text_views == 0 || !callback_cardinality_valid {
            return None;
        }
        result.len = u8::try_from(source.len()).ok()?;
        result.revision = revision;
        Some(result)
    }

    pub fn nodes(&self) -> &[AndroidInstalledActivitySceneNode] {
        &self.nodes[..usize::from(self.len)]
    }

    pub const fn len(self) -> u8 {
        self.len
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn is_active(self) -> bool {
        self.len != 0 && self.revision != 0
    }

    pub fn find_by_id(&self, id: u32) -> Option<(u8, &AndroidInstalledActivitySceneNode)> {
        if id == 0 {
            return None;
        }
        self.nodes()
            .iter()
            .enumerate()
            .find(|(_, node)| node.id == id)
            .and_then(|(index, node)| Some((u8::try_from(index).ok()?, node)))
    }

    pub fn callback_button_id(&self) -> Option<u32> {
        self.nodes()
            .iter()
            .find(|node| node.kind == AndroidSceneViewKind::Button && node.callback_registered)
            .map(|node| node.id)
    }

    pub fn callback_button_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.nodes()
            .iter()
            .filter(|node| node.kind == AndroidSceneViewKind::Button && node.callback_registered)
            .map(|node| node.id)
    }

    pub fn is_callback_button(&self, id: u32) -> bool {
        id != 0 && self.callback_button_ids().any(|candidate| candidate == id)
    }

    pub fn callback_button_count(&self) -> u8 {
        self.callback_button_ids()
            .count()
            .try_into()
            .expect("scene node capacity fits u8")
    }

    pub fn update_text(&mut self, id: u32, text: &str, revision: u64) -> bool {
        if revision <= self.revision {
            return false;
        }
        let Some((index, current)) = self.find_by_id(id) else {
            return false;
        };
        if current.kind != AndroidSceneViewKind::TextView {
            return false;
        }
        let Some(text) = AndroidSceneText::from_ascii(text) else {
            return false;
        };
        let next = AndroidInstalledActivitySceneNode { text, ..*current };
        self.nodes[usize::from(index)] = next;
        self.revision = revision;
        true
    }
}

impl Default for AndroidInstalledActivitySceneState {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AndroidInstalledActivitySceneNode, AndroidInstalledActivitySceneState,
        AndroidSceneLayoutSize, AndroidSceneOrientation, AndroidSceneViewKind,
    };

    fn profile_nodes() -> [AndroidInstalledActivitySceneNode; 5] {
        [
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::LinearLayout,
                None,
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneOrientation::Vertical,
                "",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(0),
                3,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Account profile",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::LinearLayout,
                Some(0),
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::Vertical,
                "",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::TextView,
                Some(2),
                2,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Profile status: pending",
                false,
            )
            .unwrap(),
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::Button,
                Some(2),
                1,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Verify profile",
                true,
            )
            .unwrap(),
        ]
    }

    #[test]
    fn nested_scene_is_bounded_and_only_text_updates() {
        let nodes = profile_nodes();
        let mut state = AndroidInstalledActivitySceneState::try_new(&nodes, 1).unwrap();
        assert_eq!(state.len(), 5);
        assert_eq!(state.callback_button_id(), Some(1));
        assert!(!state.update_text(1, "not a TextView", 2));
        assert!(!state.update_text(2, "stale", 1));
        assert!(state.update_text(2, "Profile status: verified", 2));
        assert_eq!(
            state.find_by_id(2).unwrap().1.text(),
            "Profile status: verified"
        );
        assert_eq!(state.find_by_id(3).unwrap().1.text(), "Account profile");
    }

    #[test]
    fn malformed_tree_duplicate_ids_and_callback_cardinality_fail_closed() {
        let nodes = profile_nodes();
        let mut bad_parent = nodes;
        bad_parent[3].parent = Some(4);
        assert!(AndroidInstalledActivitySceneState::try_new(&bad_parent, 1).is_none());

        let mut duplicate = nodes;
        duplicate[3].id = duplicate[1].id;
        assert!(AndroidInstalledActivitySceneState::try_new(&duplicate, 1).is_none());

        let mut no_callback = nodes;
        no_callback[4].callback_registered = false;
        assert!(AndroidInstalledActivitySceneState::try_new(&no_callback, 1).is_none());

        let mut second_callback = nodes;
        second_callback[1].kind = AndroidSceneViewKind::Button;
        second_callback[1].callback_registered = true;
        #[cfg(not(feature = "androidbox-multiaction3"))]
        assert!(AndroidInstalledActivitySceneState::try_new(&second_callback, 1).is_none());
        #[cfg(feature = "androidbox-multiaction3")]
        {
            let scene = AndroidInstalledActivitySceneState::try_new(&second_callback, 1).unwrap();
            assert_eq!(scene.callback_button_count(), 2);
            assert!(scene.is_callback_button(1));
            assert!(scene.is_callback_button(3));
        }
    }

    #[cfg(all(
        feature = "androidbox-layout-row14",
        not(feature = "androidbox-layout-mixed19")
    ))]
    #[test]
    fn nested_horizontal_layout_is_admitted_but_horizontal_root_is_not() {
        let mut nodes = profile_nodes();
        nodes[2] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
        )
        .unwrap();
        let state = AndroidInstalledActivitySceneState::try_new(&nodes, 1).unwrap();
        assert_eq!(
            state.nodes()[2].orientation(),
            AndroidSceneOrientation::Horizontal
        );

        nodes[0] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::LinearLayout,
            None,
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
        )
        .unwrap();
        assert!(AndroidInstalledActivitySceneState::try_new(&nodes, 2).is_none());
    }

    #[cfg(feature = "androidbox-layout-mixed19")]
    #[test]
    fn mixed_horizontal_children_require_fixed_exact_or_weighted_zero_width() {
        let mut nodes = profile_nodes();
        nodes[2] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
        )
        .unwrap();
        nodes[3] = AndroidInstalledActivitySceneNode::try_new_sized(
            AndroidSceneViewKind::Button,
            Some(2),
            2,
            AndroidSceneLayoutSize::Exact,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Fixed",
            false,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            112,
            0,
        )
        .unwrap();
        nodes[4] = AndroidInstalledActivitySceneNode::try_new_sized(
            AndroidSceneViewKind::Button,
            Some(2),
            1,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Weighted",
            true,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        let state = AndroidInstalledActivitySceneState::try_new(&nodes, 1).unwrap();
        assert_eq!(state.nodes()[3].exact_width_dp(), 112);
        assert_eq!(state.nodes()[4].layout_weight(), 1);

        let mut noncanonical = nodes;
        noncanonical[3].width = AndroidSceneLayoutSize::MatchParent;
        noncanonical[3].exact_width_dp = 0;
        assert!(AndroidInstalledActivitySceneState::try_new(&noncanonical, 2).is_none());

        let mut wrong_parent = nodes;
        wrong_parent[2].orientation = AndroidSceneOrientation::Vertical;
        assert!(AndroidInstalledActivitySceneState::try_new(&wrong_parent, 2).is_none());
    }

    #[cfg(all(
        feature = "androidbox-layout-weight15",
        not(feature = "androidbox-layout-mixed19")
    ))]
    #[test]
    fn weighted_nodes_are_bounded_and_require_a_horizontal_parent() {
        assert!(
            AndroidInstalledActivitySceneNode::try_new(
                AndroidSceneViewKind::Button,
                Some(2),
                1,
                AndroidSceneLayoutSize::Zero,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Weighted",
                true,
            )
            .is_none()
        );
        assert!(
            AndroidInstalledActivitySceneNode::try_new_weighted(
                AndroidSceneViewKind::TextView,
                Some(2),
                1,
                AndroidSceneLayoutSize::Zero,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Weighted",
                false,
                1,
            )
            .is_none()
        );
        assert!(
            AndroidInstalledActivitySceneNode::try_new_weighted(
                AndroidSceneViewKind::Button,
                Some(2),
                1,
                AndroidSceneLayoutSize::Zero,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Weighted",
                true,
                9,
            )
            .is_none()
        );

        let mut nodes = profile_nodes();
        nodes[2] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
        )
        .unwrap();
        nodes[4] = AndroidInstalledActivitySceneNode::try_new_weighted(
            AndroidSceneViewKind::Button,
            Some(2),
            1,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Weighted",
            true,
            8,
        )
        .unwrap();
        let state = AndroidInstalledActivitySceneState::try_new(&nodes, 1).unwrap();
        assert_eq!(state.nodes()[4].layout_weight(), 8);

        nodes[2] = AndroidInstalledActivitySceneNode::try_new(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Vertical,
            "",
            false,
        )
        .unwrap();
        assert!(AndroidInstalledActivitySceneState::try_new(&nodes, 2).is_none());
    }

    #[cfg(feature = "androidbox-layout-spacing16")]
    #[test]
    fn spacing_is_bounded_and_owned_only_by_supported_view_kinds() {
        let row = AndroidInstalledActivitySceneNode::try_new_spaced(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
            0,
            0,
            4,
        )
        .unwrap();
        let button = AndroidInstalledActivitySceneNode::try_new_spaced(
            AndroidSceneViewKind::Button,
            Some(2),
            1,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Spaced",
            true,
            1,
            2,
            0,
        )
        .unwrap();
        assert_eq!(row.padding_dp(), 4);
        assert_eq!(button.layout_margin_dp(), 2);

        for malformed in [
            AndroidInstalledActivitySceneNode::try_new_spaced(
                AndroidSceneViewKind::TextView,
                Some(0),
                2,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Margin",
                false,
                0,
                1,
                0,
            ),
            AndroidInstalledActivitySceneNode::try_new_spaced(
                AndroidSceneViewKind::Button,
                Some(2),
                1,
                AndroidSceneLayoutSize::Zero,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Padding",
                true,
                1,
                0,
                1,
            ),
            AndroidInstalledActivitySceneNode::try_new_spaced(
                AndroidSceneViewKind::LinearLayout,
                Some(0),
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::Horizontal,
                "",
                false,
                0,
                0,
                17,
            ),
        ] {
            assert!(malformed.is_none());
        }
    }

    #[cfg(feature = "androidbox-layout-directional17")]
    #[test]
    fn directional_spacing_preserves_each_edge_and_fails_closed() {
        let row = AndroidInstalledActivitySceneNode::try_new_directional(
            AndroidSceneViewKind::LinearLayout,
            Some(0),
            0,
            AndroidSceneLayoutSize::MatchParent,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::Horizontal,
            "",
            false,
            0,
            0,
            0,
            0,
            0,
            6,
            4,
            2,
            8,
        )
        .unwrap();
        let button = AndroidInstalledActivitySceneNode::try_new_directional(
            AndroidSceneViewKind::Button,
            Some(2),
            1,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Directional",
            true,
            1,
            2,
            1,
            4,
            3,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        assert_eq!(
            [
                row.padding_left_dp(),
                row.padding_top_dp(),
                row.padding_right_dp(),
                row.padding_bottom_dp(),
            ],
            [6, 4, 2, 8]
        );
        assert_eq!(
            [
                button.layout_margin_left_dp(),
                button.layout_margin_top_dp(),
                button.layout_margin_right_dp(),
                button.layout_margin_bottom_dp(),
            ],
            [2, 1, 4, 3]
        );

        assert!(
            AndroidInstalledActivitySceneNode::try_new_directional(
                AndroidSceneViewKind::LinearLayout,
                Some(0),
                0,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::Horizontal,
                "",
                false,
                0,
                0,
                0,
                0,
                0,
                17,
                0,
                0,
                0,
            )
            .is_none()
        );
        assert!(
            AndroidInstalledActivitySceneNode::try_new_directional(
                AndroidSceneViewKind::TextView,
                Some(0),
                2,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Margin",
                false,
                0,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            )
            .is_none()
        );
    }

    #[cfg(feature = "androidbox-layout-size18")]
    #[test]
    fn exact_dp_sizes_are_canonical_bounded_scene_data() {
        let title = AndroidInstalledActivitySceneNode::try_new_sized(
            AndroidSceneViewKind::TextView,
            Some(0),
            1,
            AndroidSceneLayoutSize::Exact,
            AndroidSceneLayoutSize::WrapContent,
            AndroidSceneOrientation::None,
            "Exact title",
            false,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            240,
            0,
        )
        .unwrap();
        let button = AndroidInstalledActivitySceneNode::try_new_sized(
            AndroidSceneViewKind::Button,
            Some(2),
            2,
            AndroidSceneLayoutSize::Zero,
            AndroidSceneLayoutSize::Exact,
            AndroidSceneOrientation::None,
            "Exact button",
            true,
            1,
            2,
            1,
            4,
            3,
            0,
            0,
            0,
            0,
            0,
            64,
        )
        .unwrap();
        assert_eq!(title.exact_width_dp(), 240);
        assert_eq!(title.exact_height_dp(), 0);
        assert_eq!(button.exact_width_dp(), 0);
        assert_eq!(button.exact_height_dp(), 64);

        for malformed in [
            AndroidInstalledActivitySceneNode::try_new_sized(
                AndroidSceneViewKind::TextView,
                Some(0),
                1,
                AndroidSceneLayoutSize::Exact,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Missing size",
                false,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
            AndroidInstalledActivitySceneNode::try_new_sized(
                AndroidSceneViewKind::TextView,
                Some(0),
                1,
                AndroidSceneLayoutSize::MatchParent,
                AndroidSceneLayoutSize::WrapContent,
                AndroidSceneOrientation::None,
                "Hidden size",
                false,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                0,
            ),
        ] {
            assert!(malformed.is_none());
        }
    }
}
