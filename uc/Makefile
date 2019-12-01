all: fib

fib: fib.c
	$(CC) -Iunicorn/include $(CFLAGS) $^ -o $@ unicorn/libunicorn.a -lpthread -lm

.PHONY: clean
clean:
	$(RM) fib
