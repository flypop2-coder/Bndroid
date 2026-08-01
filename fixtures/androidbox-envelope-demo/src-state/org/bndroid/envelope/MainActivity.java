package org.bndroid.envelope;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Persistent Activity-state fixture: retain a TextView reference and update
 * an integer field across successive click callbacks.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    private TextView statusView;
    private int clickCount;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_envelope);
        statusView = (TextView) findViewById(R.id.status);
        clickCount = 0;
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
        clickCount++;
        statusView.setText(statusTextFor(view));
    }
}
