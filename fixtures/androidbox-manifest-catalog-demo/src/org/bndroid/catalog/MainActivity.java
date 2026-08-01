package org.bndroid.catalog;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Executable launcher for the component-catalog fixture.
 *
 * The additional Activity, alias, service, receiver, and provider declarations
 * intentionally exercise package metadata only. AndroidBox must not claim that
 * those components execute until their framework contracts are implemented.
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_catalog);
        findViewById(R.id.action_components).setOnClickListener(this);
        findViewById(R.id.action_permissions).setOnClickListener(this);
    }

    @Override
    public void onClick(View view) {
        int viewId = view.getId();
        TextView status = (TextView) findViewById(R.id.status);
        if (viewId == R.id.action_components) {
            status.setText(R.string.status_components);
        } else if (viewId == R.id.action_permissions) {
            status.setText(R.string.status_permissions);
        }
    }
}
