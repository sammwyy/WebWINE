#include <windows.h>
#include <commdlg.h>

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hInstance;
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    char filename[MAX_PATH];
    ZeroMemory(filename, sizeof(filename));
    lstrcpyA(filename, "webwine_output.txt");

    OPENFILENAMEA ofn;
    ZeroMemory(&ofn, sizeof(ofn));
    ofn.lStructSize = sizeof(ofn);
    ofn.hwndOwner = NULL;
    ofn.lpstrFilter = "Text Files\0*.txt\0All Files\0*.*\0";
    ofn.lpstrFile = filename;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_OVERWRITEPROMPT;
    ofn.lpstrTitle = "05_save_file_dialog";
    ofn.lpstrDefExt = "txt";

    if (GetSaveFileNameA(&ofn)) {
        MessageBoxA(NULL, filename, "Save path", MB_OK);
        return 0;
    }

    MessageBoxA(NULL, "Save cancelled.", "05_save_file_dialog", MB_OK);
    return 1;
}
