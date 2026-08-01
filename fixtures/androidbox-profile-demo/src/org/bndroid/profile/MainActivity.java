package org.bndroid.profile;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Second standard-API Activity fixture for the bounded SceneRPC-2 profile.
 *
 * The extra title TextView and nested LinearLayout deliberately make this
 * scene structurally different from InteractiveActivity-1. The callback
 * mutates only the status TextView.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_profile);
        findViewById(R.id.action).setOnClickListener(this);
    }

    @Override
    public void onClick(View view) {
        TextView status = (TextView) findViewById(R.id.status);
        status.setText(R.string.status_after);
    }
}
