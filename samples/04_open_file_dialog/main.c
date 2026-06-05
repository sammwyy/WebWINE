#include <windows.h>
#include <commdlg.h>

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hInstance;
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    char filename[MAX_PATH];
    ZeroMemory(filename, sizeof(filename));

    OPENFILENAMEA ofn;
    ZeroMemory(&ofn, sizeof(ofn));
    ofn.lStructSize = sizeof(ofn);
    ofn.hwndOwner = NULL;
    ofn.lpstrFilter = "Text Files\0*.txt\0All Files\0*.*\0";
    ofn.lpstrFile = filename;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
    ofn.lpstrTitle = "04_open_file_dialog";

    if (GetOpenFileNameA(&ofn)) {
        MessageBoxA(NULL, filename, "Selected file", MB_OK);
        return 0;
    }

    MessageBoxA(NULL, "No file selected or dialog cancelled.", "04_open_file_dialog", MB_OK);
    return 1;
}
