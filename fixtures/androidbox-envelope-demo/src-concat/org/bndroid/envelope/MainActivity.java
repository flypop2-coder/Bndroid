package org.bndroid.envelope;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Dynamic-string fixture: retained integer state is appended to an
 * APK-owned prefix before the result is passed to TextView.setText.
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

    @Override
    public void onClick(View view) {
        int next = clickCount + 1;
        clickCount = next;
        statusView.setText("Envelope count: " + next);
    }
}
