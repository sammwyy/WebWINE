#include <windows.h>
#include <commctrl.h>

#define ID_BUTTON 2001
#define ID_EDIT   2002
#define ID_LIST   2003

static const char *WINDOW_CLASS_NAME = "WebWINE_10_CommonControls";

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    (void)lParam;

    switch (msg) {
        case WM_CREATE: {
            CreateWindowExA(0, "STATIC", "Type something:", WS_CHILD | WS_VISIBLE,
                            20, 20, 160, 24, hwnd, NULL, NULL, NULL);

            CreateWindowExA(WS_EX_CLIENTEDGE, "EDIT", "hello", WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL,
                            20, 48, 220, 26, hwnd, (HMENU)ID_EDIT, NULL, NULL);

            CreateWindowExA(0, "BUTTON", "Read Edit", WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                            260, 48, 120, 28, hwnd, (HMENU)ID_BUTTON, NULL, NULL);

            HWND list = CreateWindowExA(WS_EX_CLIENTEDGE, "LISTBOX", NULL, WS_CHILD | WS_VISIBLE | LBS_STANDARD,
                                        20, 92, 220, 120, hwnd, (HMENU)ID_LIST, NULL, NULL);
            SendMessageA(list, LB_ADDSTRING, 0, (LPARAM)"First item");
            SendMessageA(list, LB_ADDSTRING, 0, (LPARAM)"Second item");
            SendMessageA(list, LB_ADDSTRING, 0, (LPARAM)"Third item");
            return 0;
        }
        case WM_COMMAND:
            if (LOWORD(wParam) == ID_BUTTON) {
                char text[256];
                HWND edit = GetDlgItem(hwnd, ID_EDIT);
                GetWindowTextA(edit, text, sizeof(text));
                MessageBoxA(hwnd, text, "Edit content", MB_OK);
                return 0;
            }
            return 0;
        case WM_DESTROY:
            PostQuitMessage(0);
            return 0;
        default:
            return DefWindowProcA(hwnd, msg, wParam, lParam);
    }
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hPrevInstance;
    (void)lpCmdLine;

    INITCOMMONCONTROLSEX icc;
    icc.dwSize = sizeof(icc);
    icc.dwICC = ICC_STANDARD_CLASSES;
    InitCommonControlsEx(&icc);

    WNDCLASSA wc;
    ZeroMemory(&wc, sizeof(wc));
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = WINDOW_CLASS_NAME;
    wc.hCursor = LoadCursorA(NULL, IDC_ARROW);
    wc.hbrBackground = (HBRUSH)(COLOR_BTNFACE + 1);
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(0, WINDOW_CLASS_NAME, "10_common_controls", WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, 480, 300,
                                NULL, NULL, hInstance, NULL);
    if (!hwnd) return 1;

    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }

    return (int)msg.wParam;
}
