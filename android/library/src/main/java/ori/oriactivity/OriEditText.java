package ori.oriactivity;

import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.EditorInfo;
import android.text.InputType;
import android.content.Context;
import android.widget.EditText;

public class OriEditText extends EditText {
    public InputConnection inputConnection;

    public OriEditText(Context context) {
        super(context);
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
        outAttrs.actionLabel = null;
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT;
        outAttrs.imeOptions = EditorInfo.IME_ACTION_DONE;

        InputConnection editTextInputConnection = super.onCreateInputConnection(outAttrs);

        inputConnection = new BaseInputConnection(this, true) {
            @Override
            public boolean commitText(CharSequence text, int newCursorPosition) {
                nativeCommitText(text.toString(), newCursorPosition);
                return editTextInputConnection.commitText(text, newCursorPosition);
            }

            @Override
            public boolean setComposingText(CharSequence text, int newCursorPosition) {
                nativeSetComposingText(text.toString(), newCursorPosition);
                return editTextInputConnection.setComposingText(text, newCursorPosition);
            }

            @Override
            public boolean deleteSurroundingText(int beforeLength, int afterLength) {
                nativeDeleteSurroundingText(beforeLength, afterLength);
                return editTextInputConnection.deleteSurroundingText(beforeLength, afterLength);
            }
        };

        return inputConnection;
    }

    public native void nativeCommitText(String text, int newCursorPosition);
    public native void nativeSetComposingText(String text, int newCursorPosition);
    public native void nativeDeleteSurroundingText(int beforeLength, int afterLength);
}
