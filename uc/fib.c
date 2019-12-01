#include <stdio.h>
#include <stdint.h>
#include <time.h>

#include "unicorn/unicorn.h"

#define HOOKS 1

#define CODE_ADDRESS  0x41410000
#define STACK_ADDRESS 0x12345000

#define CODE (\
"\x6a\x00\x6a\x00\x6a\x01\x58\x5b\x59\x48\x89\xc2\x48\x01\xd8\x48" \
"\x89\xd3\x48\xff\xc1\x51\x53\x50\x48\x81\xf9\xff\xff\xff\x00\x75" \
"\xe5\x90")

uint64_t ins = 0;
uint64_t reads = 0;
uint64_t writes = 0;

uint64_t diff_ms(struct timespec *start, struct timespec *end) {
	time_t secs = end->tv_sec - start->tv_sec;
	long ns = end->tv_nsec - start->tv_nsec;

	return secs * 1000 + ns / 1000000;
}

void hook_code(uc_engine *uc, uint64_t addr, uint32_t size, void *user_data) {
	++ins;

	uint64_t r_rip;
	uc_reg_read(uc, UC_X86_REG_RIP, &r_rip);

	if (r_rip == CODE_ADDRESS + sizeof(CODE) - 1) {
		uc_emu_stop(uc);
	}
}

void hook_mem(
	uc_engine *uc,
	uc_mem_type type,
	uint64_t address,
	int size,
	int64_t value,
	void *user_data
) {
	if (type == UC_MEM_WRITE)
		++writes;
	else
		++reads;
}

int main(void) {
	uc_engine *uc;
	uc_err err;

	err = uc_open(UC_ARCH_X86, UC_MODE_64, &uc);
	if (err != UC_ERR_OK) {
		printf("Failed on uc_open() with error returned: %u\n", err);
		return -1;
	}

	printf("mapping stack...\n");
	err = uc_mem_map(uc, STACK_ADDRESS, 0x1000, UC_PROT_ALL);
	if (err != UC_ERR_OK) {
		printf("Failed to write emulation code to memory, quit!\n");
		return -1;
	}

	uint64_t r_rsp = STACK_ADDRESS + 0x800;
	uc_reg_write(uc, UC_X86_REG_RSP, &r_rsp);

	printf("mapping text...\n");
	err = uc_mem_map(uc, CODE_ADDRESS, 0x1000, UC_PROT_ALL);
	if (err != UC_ERR_OK) {
		printf("Failed to write emulation code to memory, quit!\n");
		return -1;
	}

	 err = uc_mem_write(uc, CODE_ADDRESS, CODE, sizeof(CODE) - 1);
	 if (err != UC_ERR_OK) {
		 printf("Failed to write emulation code to memory, quit!\n");
		 return -1;
	 }

#if HOOKS
	 uc_hook ins_hook, mem_hook;
	 printf("installing ins hook...\n");
	 err = uc_hook_add(uc, &ins_hook, UC_HOOK_CODE, hook_code, NULL, 1, 0);
	 if (err != UC_ERR_OK) {
		 printf("Failed to install code hook\n");
		 return -1;
	 }

	 printf("installing mem hook...\n");
	 err = uc_hook_add(uc, &mem_hook, UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, hook_mem, NULL, 1, 0);
	 if (err != UC_ERR_OK) {
		 printf("Failed to install mem hook\n");
		 return -1;
	 }
#endif

	 struct timespec start, end;

	 printf("starting emulation....\n");

	 clock_gettime(CLOCK_MONOTONIC_RAW, &start);
	 err = uc_emu_start(uc, CODE_ADDRESS, CODE_ADDRESS + sizeof(CODE) - 1, 0, 0);
	 clock_gettime(CLOCK_MONOTONIC_RAW, &end);

	if (err != UC_ERR_OK) {
		printf("Failed on uc_emu_start() with error returned %u: %s\n", err, uc_strerror(err));
		return -1;
	}

	uint64_t r_rax;
	uint64_t r_rcx;

	uc_reg_read(uc, UC_X86_REG_RAX, &r_rax);
	uc_reg_read(uc, UC_X86_REG_RCX, &r_rcx);

	float delta = diff_ms(&start, &end) / (float)1000;

	printf("result in rax is %llx, %lld loops\n", r_rax, r_rcx);
	printf(
		"emulated %lld ins with %lld mem reads and %lld mem writes in %.2f secs, %.2f mips\n",
		ins,
		reads,
		writes,
		delta,
		(float)ins / delta / (float)1000000
	);
}
