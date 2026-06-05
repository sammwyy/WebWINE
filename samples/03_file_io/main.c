#include <windows.h>

static void show_last_error(const char *operation) {
    char buffer[256];
    wsprintfA(buffer, "%s failed. GetLastError=%lu", operation, GetLastError());
    MessageBoxA(NULL, buffer, "03_file_io error", MB_OK | MB_ICONERROR);
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hInstance;
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    const char *path = "webwine_file_io_test.txt";
    const char *text = "Hello from CreateFileA + WriteFile + ReadFile.\r\n";

    HANDLE file = CreateFileA(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) {
        show_last_error("CreateFileA/write");
        return 1;
    }

    DWORD written = 0;
    if (!WriteFile(file, text, lstrlenA(text), &written, NULL)) {
        CloseHandle(file);
        show_last_error("WriteFile");
        return 1;
    }
    CloseHandle(file);

    file = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) {
        show_last_error("CreateFileA/read");
        return 1;
    }

    char buffer[256];
    ZeroMemory(buffer, sizeof(buffer));
    DWORD read = 0;
    if (!ReadFile(file, buffer, sizeof(buffer) - 1, &read, NULL)) {
        CloseHandle(file);
        show_last_error("ReadFile");
        return 1;
    }
    CloseHandle(file);

    MessageBoxA(NULL, buffer, "03_file_io readback", MB_OK);
    return 0;
}
