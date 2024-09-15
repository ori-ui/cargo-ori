package ori.oriactivity;

import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.CompletionInfo;
import android.view.inputmethod.InputContentInfo;
import android.view.inputmethod.CorrectionInfo;
import android.view.inputmethod.TextAttribute;
import android.view.inputmethod.ExtractedTextRequest;
import android.view.inputmethod.ExtractedText;
import android.text.InputType;
import android.view.KeyEvent;
import android.content.Context;
import android.widget.EditText;
import android.os.Bundle;

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
            public boolean beginBatchEdit() {
                return editTextInputConnection.beginBatchEdit();
            }

            @Override
            public boolean clearMetaKeyStates(int states) {
                return editTextInputConnection.clearMetaKeyStates(states);
            }

            @Override
            public boolean commitCompletion(CompletionInfo text) {
                return editTextInputConnection.commitCompletion(text);
            }

            @Override
            public boolean commitCorrection(CorrectionInfo correctionInfo) {
                return editTextInputConnection.commitCorrection(correctionInfo);
            }

            @Override
            public boolean commitText(CharSequence text, int newCursorPosition) {
                nativeCommitText(text.toString(), newCursorPosition);
                return editTextInputConnection.commitText(text, newCursorPosition);
            } 

            @Override
            public boolean deleteSurroundingText(int beforeLength, int afterLength) {
                nativeDeleteSurroundingText(beforeLength, afterLength);
                return editTextInputConnection.deleteSurroundingText(beforeLength, afterLength);
            }

            @Override
            public boolean endBatchEdit() {
                return editTextInputConnection.endBatchEdit();
            }
            
            @Override
            public boolean finishComposingText() {
                return editTextInputConnection.finishComposingText();
            }
            
            @Override
            public int getCursorCapsMode(int reqModes) {
                return editTextInputConnection.getCursorCapsMode(reqModes);
            }

            @Override
            public ExtractedText getExtractedText(ExtractedTextRequest request, int flags) {
                return editTextInputConnection.getExtractedText(request, flags);
            }

            @Override
            public CharSequence getSelectedText(int flags) {
                return editTextInputConnection.getSelectedText(flags);
            }

            @Override
            public CharSequence getTextAfterCursor(int length, int flags) {
                return editTextInputConnection.getTextAfterCursor(length, flags);
            }

            @Override
            public CharSequence getTextBeforeCursor(int length, int flags) {
                return editTextInputConnection.getTextBeforeCursor(length, flags);
            }

            @Override
            public boolean performContextMenuAction(int id) {
                return editTextInputConnection.performContextMenuAction(id);
            }

            @Override
            public boolean performEditorAction(int actionCode) {
                return editTextInputConnection.performEditorAction(actionCode);
            }

            @Override
            public boolean performPrivateCommand(String action, Bundle data) {
                return editTextInputConnection.performPrivateCommand(action, data);
            }

            @Override
            public boolean reportFullscreenMode(boolean enabled) {
                return editTextInputConnection.reportFullscreenMode(enabled);
            }

            @Override
            public boolean requestCursorUpdates(int cursorUpdateMode) {
                return editTextInputConnection.requestCursorUpdates(cursorUpdateMode);
            }

            @Override
            public boolean sendKeyEvent(android.view.KeyEvent event) {
                return editTextInputConnection.sendKeyEvent(event);
            }

            @Override
            public boolean setComposingRegion(int start, int end) {
                return editTextInputConnection.setComposingRegion(start, end);
            }

            @Override
            public boolean setComposingText(CharSequence text, int newCursorPosition) {
                nativeSetComposingText(text.toString(), newCursorPosition);
                return editTextInputConnection.setComposingText(text, newCursorPosition);
            }

            @Override
            public boolean setSelection(int start, int end) {
                return editTextInputConnection.setSelection(start, end);
            }
        };

        return inputConnection;
    }

    public native void nativeCommitText(String text, int newCursorPosition);
    public native void nativeSetComposingText(String text, int newCursorPosition);
    public native void nativeDeleteSurroundingText(int beforeLength, int afterLength);
}
