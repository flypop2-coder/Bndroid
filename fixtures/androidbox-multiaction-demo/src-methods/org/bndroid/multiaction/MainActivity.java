package org.bndroid.multiaction;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * ABI58 fixture: the callback delegates its result selection to APK-owned
 * code instead of embedding the complete branch in onClick.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_multiaction);
        findViewById(R.id.action_approve).setOnClickListener(this);
        findViewById(R.id.action_reject).setOnClickListener(this);
    }

    private static int statusTextFor(int viewId) {
        return viewId == R.id.action_approve
                ? R.string.status_approved
                : R.string.status_rejected;
    }

    @Override
    public void onClick(View view) {
        int viewId = view.getId();
        TextView status = (TextView) findViewById(R.id.status);
        status.setText(statusTextFor(viewId));
    }
}
