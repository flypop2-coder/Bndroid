package org.bndroid.macdemo;

import android.app.Activity;
import android.os.Bundle;

/** Launcher Activity whose only visible content comes from compiled APK resources. */
public final class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_main);
    }
}
