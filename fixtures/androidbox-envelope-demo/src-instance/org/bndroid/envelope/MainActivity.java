package org.bndroid.envelope;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * ABI59 fixture: pass the clicked View object to an APK-owned Activity
 * instance helper. The helper performs the framework getId call.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_envelope);
        findViewById(R.id.action_approve).setOnClickListener(this);
        findViewById(R.id.action_reject).setOnClickListener(this);
    }

    private int statusTextFor(View view) {
        return view.getId() == R.id.action_approve
                ? R.string.status_approved
                : R.string.status_rejected;
    }

    @Override
    public void onClick(View view) {
        TextView status = (TextView) findViewById(R.id.status);
        status.setText(statusTextFor(view));
    }
}
