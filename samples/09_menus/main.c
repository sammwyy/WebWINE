#include <windows.h>

#define ID_MENU_OPEN 1001
#define ID_MENU_EXIT 1002

static const char *WINDOW_CLASS_NAME = "WebWINE_09_Menus";

static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    (void)lParam;

    switch (msg) {
        case WM_COMMAND:
            switch (LOWORD(wParam)) {
                case ID_MENU_OPEN:
                    MessageBoxA(hwnd, "Open menu item clicked.", "09_menus", MB_OK);
                    return 0;
                case ID_MENU_EXIT:
                    PostQuitMessage(0);
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

static HMENU create_main_menu(void) {
    HMENU menu = CreateMenu();
    HMENU fileMenu = CreatePopupMenu();

    AppendMenuA(fileMenu, MF_STRING, ID_MENU_OPEN, "Open");
    AppendMenuA(fileMenu, MF_SEPARATOR, 0, NULL);
    AppendMenuA(fileMenu, MF_STRING, ID_MENU_EXIT, "Exit");
    AppendMenuA(menu, MF_POPUP, (UINT_PTR)fileMenu, "File");

    return menu;
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hPrevInstance;
    (void)lpCmdLine;

    WNDCLASSA wc;
    ZeroMemory(&wc, sizeof(wc));
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = WINDOW_CLASS_NAME;
    wc.hCursor = LoadCursorA(NULL, IDC_ARROW);
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(0, WINDOW_CLASS_NAME, "09_menus", WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, 640, 360,
                                NULL, NULL, hInstance, NULL);
    if (!hwnd) return 1;

    SetMenu(hwnd, create_main_menu());
    ShowWindow(hwnd, nCmdShow);
    UpdateWindow(hwnd);

    MSG msg;
    while (GetMessageA(&msg, NULL, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessageA(&msg);
    }

    return (int)msg.wParam;
}
