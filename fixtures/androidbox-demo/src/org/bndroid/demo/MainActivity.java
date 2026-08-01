package org.bndroid.demo;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

/**
 * Conventional Android launcher declaration for tooling inspection. AndroidBox
 * DEX-0 does not execute this class or emulate ActivityThread.
 */
public final class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        TextView label = new TextView(this);
        label.setText("AndroidBox DEX-0 fixture");
        setContentView(label);
    }
}
