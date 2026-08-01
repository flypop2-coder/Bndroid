package org.bndroid.envelope;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Two-action Java fixture wrapped in a realistic mixed-method APK envelope.
 *
 * The extra asset is intentionally unreachable from this Activity. AndroidBox
 * must admit the supported compiled Activity without interpreting unrelated
 * DEFLATE archive content.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_envelope);
        findViewById(R.id.action_approve).setOnClickListener(this);
        findViewById(R.id.action_reject).setOnClickListener(this);
    }

    @Override
    public void onClick(View view) {
        int viewId = view.getId();
        TextView status = (TextView) findViewById(R.id.status);
        if (viewId == R.id.action_approve) {
            status.setText(R.string.status_approved);
        } else if (viewId == R.id.action_reject) {
            status.setText(R.string.status_rejected);
        }
    }
}
