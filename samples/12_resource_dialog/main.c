#include <windows.h>

#define IDD_MAIN_DIALOG 101
#define IDC_HELLO_TEXT 1001

static INT_PTR CALLBACK DialogProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    (void)lParam;

    switch (msg) {
        case WM_INITDIALOG:
            SetDlgItemTextA(hwnd, IDC_HELLO_TEXT, "Resource dialog loaded from PE resources.");
            return TRUE;
        case WM_COMMAND:
            switch (LOWORD(wParam)) {
                case IDOK:
                case IDCANCEL:
                    EndDialog(hwnd, LOWORD(wParam));
                    return TRUE;
            }
            return FALSE;
        default:
            return FALSE;
    }
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    INT_PTR result = DialogBoxParamA(hInstance, MAKEINTRESOURCEA(IDD_MAIN_DIALOG), NULL, DialogProc, 0);
    if (result == IDOK) {
        MessageBoxA(NULL, "Dialog returned IDOK.", "12_resource_dialog", MB_OK);
        return 0;
    }

    MessageBoxA(NULL, "Dialog cancelled or failed.", "12_resource_dialog", MB_OK);
    return 1;
}
