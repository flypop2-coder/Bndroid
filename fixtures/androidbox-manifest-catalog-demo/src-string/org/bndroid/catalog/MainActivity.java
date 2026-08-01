package org.bndroid.catalog;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/**
 * Direct-string fixture: retained integer state selects a DEX string literal
 * passed to TextView.setText(CharSequence).
 */
public final class MainActivity extends Activity implements View.OnClickListener {
    private TextView statusView;
    private int clickCount;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_catalog);
        statusView = (TextView) findViewById(R.id.status);
        clickCount = 0;
        findViewById(R.id.action_components).setOnClickListener(this);
        findViewById(R.id.action_permissions).setOnClickListener(this);
    }

    @Override
    public void onClick(View view) {
        int next = clickCount + 1;
        clickCount = next;
        statusView.setText(next == 1
                ? "First review recorded"
                : "Review state advanced again");
    }
}
