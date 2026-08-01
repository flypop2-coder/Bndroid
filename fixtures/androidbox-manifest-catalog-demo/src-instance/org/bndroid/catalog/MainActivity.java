package org.bndroid.catalog;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * ABI59 fixture: pass a real View reference to an APK-owned Activity instance
 * helper. The helper performs the framework getId call.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_catalog);
        findViewById(R.id.action_components).setOnClickListener(this);
        findViewById(R.id.action_permissions).setOnClickListener(this);
    }

    private int statusTextFor(View view) {
        return view.getId() == R.id.action_components
                ? R.string.status_components
                : R.string.status_permissions;
    }

    @Override
    public void onClick(View view) {
        TextView status = (TextView) findViewById(R.id.status);
        status.setText(statusTextFor(view));
    }
}
