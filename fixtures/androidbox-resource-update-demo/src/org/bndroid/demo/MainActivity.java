package org.bndroid.demo;

import android.app.Activity;
import android.os.Bundle;

/** Android launcher whose updated view is defined entirely by APK resources. */
public final class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(R.layout.activity_main);
    }
}
