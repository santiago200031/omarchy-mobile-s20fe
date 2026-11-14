// SPDX-License-Identifier: MIT
// r8q-liveness-heartbeat: pin to one CPU and publish a monotonic tick.
//
// usage: r8q-liveness-heartbeat <cpu> <dir>
//
// Writes <dir>/cpu<cpu> once a second with the current CLOCK_MONOTONIC
// time in nanoseconds. Runs SCHED_FIFO 99 on the pinned CPU, so on a
// healthy core the file is never more than ~1 s stale. The monitor
// (r8q-liveness-monitor) reboots the phone when an online core's file
// goes stale: that is the core-bus-hang class of the RPMh/RCU crashes
// that the kernel detectors cannot see (no ticks at all on the wedged
// core). device/r8q/PORT-PLAN.md, blocker 1.
// _GNU_SOURCE: CPU_SET/CPU_ZERO, sched_setaffinity and sched_getcpu are
// GNU extensions glibc only exposes through <sched.h> when it is defined.
#define _GNU_SOURCE

#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

int main(int argc, char **argv)
{
	char path[256];
	int cpu;
	cpu_set_t set;
	struct sched_param sp;

	if (argc != 3)
		return 2;
	cpu = atoi(argv[1]);
	if (cpu < 0)
		return 2;

	CPU_ZERO(&set);
	CPU_SET(cpu, &set);
	if (sched_setaffinity(0, sizeof(set), &set)) {
		perror("sched_setaffinity");
		return 1;
	}
	sp.sched_priority = 99;
	if (sched_setscheduler(0, SCHED_FIFO, &sp))
		perror("sched_setscheduler");

	snprintf(path, sizeof(path), "%s/cpu%d", argv[2], cpu);
	FILE *f = fopen(path, "w");
	if (!f) {
		perror("fopen");
		return 1;
	}

	for (;;) {
		struct timespec ts;

		// If this CPU went offline, our tasks were migrated: pin back
		// so the heartbeat keeps meaning what the monitor expects.
		if (sched_getcpu() != cpu) {
			sched_setaffinity(0, sizeof(set), &set);
		}
		if (clock_gettime(CLOCK_MONOTONIC, &ts)) {
			perror("clock_gettime");
			return 1;
		}
		fprintf(f, "%lld\n",
			(long long)(ts.tv_sec * 1000000000LL + ts.tv_nsec));
		fflush(f);
		nanosleep(&(struct timespec){ .tv_sec = 1, .tv_nsec = 0 }, NULL);
	}
}
