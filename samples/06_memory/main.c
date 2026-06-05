#include <windows.h>

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    (void)hInstance;
    (void)hPrevInstance;
    (void)lpCmdLine;
    (void)nCmdShow;

    HANDLE heap = GetProcessHeap();
    char *heapMemory = (char *)HeapAlloc(heap, HEAP_ZERO_MEMORY, 128);
    if (!heapMemory) return 1;

    lstrcpyA(heapMemory, "HeapAlloc succeeded.");
    MessageBoxA(NULL, heapMemory, "06_memory heap", MB_OK);
    HeapFree(heap, 0, heapMemory);

    char *virtualMemory = (char *)VirtualAlloc(NULL, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!virtualMemory) return 2;

    lstrcpyA(virtualMemory, "VirtualAlloc succeeded.");
    MessageBoxA(NULL, virtualMemory, "06_memory virtual", MB_OK);
    VirtualFree(virtualMemory, 0, MEM_RELEASE);

    return 0;
}
