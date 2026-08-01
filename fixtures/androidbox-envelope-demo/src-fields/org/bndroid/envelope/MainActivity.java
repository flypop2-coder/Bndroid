package org.bndroid.envelope;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Activity-field fixture: retain the status TextView on the Activity instance
 * during onCreate and read that exact reference from the click callback.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    private TextView statusView;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_envelope);
        statusView = (TextView) findViewById(R.id.status);
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
        statusView.setText(statusTextFor(view));
    }
}
