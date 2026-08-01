package org.bndroid.catalog;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * ABI58 fixture: both callbacks execute one APK-owned helper method before
 * mutating the existing status TextView.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_catalog);
        findViewById(R.id.action_components).setOnClickListener(this);
        findViewById(R.id.action_permissions).setOnClickListener(this);
    }

    private static int statusTextFor(int viewId) {
        return viewId == R.id.action_components
                ? R.string.status_components
                : R.string.status_permissions;
    }

    @Override
    public void onClick(View view) {
        int viewId = view.getId();
        TextView status = (TextView) findViewById(R.id.status);
        status.setText(statusTextFor(viewId));
    }
}
