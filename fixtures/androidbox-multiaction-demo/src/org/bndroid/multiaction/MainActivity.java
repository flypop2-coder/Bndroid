package org.bndroid.multiaction;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Deterministic two-action fixture for a future bounded AndroidBox profile.
 *
 * Both real Android Buttons bind this Activity as their listener. The
 * callback reads the clicked View ID and updates the same status TextView
 * with a distinct compiled string resource for each admitted action.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_multiaction);
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
