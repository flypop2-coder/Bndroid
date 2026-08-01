package org.bndroid.interactive;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.TextView;

/** Small standard-API Activity used by the bounded InteractiveActivity-1 profile. */
public final class MainActivity extends Activity implements View.OnClickListener {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_main);
        findViewById(R.id.action).setOnClickListener(this);
    }

    @Override
    public void onClick(View view) {
        TextView label = (TextView) findViewById(R.id.label);
        label.setText(R.string.clicked_message);
    }
}
