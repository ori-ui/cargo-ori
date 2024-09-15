package ori.oriactivity;

import android.app.NativeActivity;
import android.view.inputmethod.InputMethodManager;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.ExtractedText;
import android.content.Context;
import android.view.View;
import android.os.Bundle;

import ori.oriactivity.OriEditText;

public class OriActivity extends NativeActivity {
    static {
        System.loadLibrary("oriapp");
    }

    private InputMethodManager imm;
    private View decorView;
    private OriEditText editText;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        decorView = getWindow().getDecorView();
        imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);

        editText = new OriEditText(this);
        editText.setVisibility(View.GONE);
        setContentView(editText);
    }

    public void showIME() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                editText.setVisibility(View.VISIBLE);
                editText.requestFocus();
                imm.showSoftInput(editText, InputMethodManager.SHOW_IMPLICIT);
            }
        }); 
    }

    public void hideIME() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                editText.setVisibility(View.GONE);
                imm.hideSoftInputFromWindow(editText.getWindowToken(), 0);
            }
        });
    }

    public void setIMEText(String text) {
        System.out.println("setIMEText: " + editText.getText().toString());
    }

    public void setIMESelection(int start, int end) {
    }
}
