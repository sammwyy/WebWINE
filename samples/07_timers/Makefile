# Auto-generated WebWINE sample Makefile
ifeq ($(origin CC), default)
CC := i686-w64-mingw32-gcc
endif
WINDRES ?= i686-w64-mingw32-windres
OUT ?= ../build/07_timers.exe
CFLAGS ?= -Wall -Wextra -std=c99 -DWIN32_LEAN_AND_MEAN
LDFLAGS ?= -mwindows
LIBS ?= -luser32

.PHONY: all clean

all: $(OUT)

$(OUT): main.c 
	mkdir -p $(dir $(OUT))
	$(CC) $(CFLAGS) main.c  -o $(OUT) $(LDFLAGS) $(LIBS)

resource.res: resource.rc
	$(WINDRES) resource.rc -O coff -o resource.res

clean:
	rm -f $(OUT) resource.res
