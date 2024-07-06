// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#define _GNU_SOURCE

#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/time.h>
#include <sys/mman.h>
#include <err.h>
#include <errno.h>
#include <pthread.h>
#include <sys/stat.h>
#include <zstd.h>
#include <stdarg.h>
#include <sys/mman.h>
#include <sys/syscall.h>

#ifdef __x86_64__
#define MEMFD_CREATE_SYSCALL_ID 319
#else
#define MEMFD_CREATE_SYSCALL_ID 279
#endif

int memfd_create_syscall(const char *name, unsigned flags)
{

  return syscall(MEMFD_CREATE_SYSCALL_ID, name, flags);
}

#define TIMESTAMP_BUFFER_SIZE 50

// Global flag to cache whether logging is enabled
static bool logEnabled = false;

// Function to initialize the logging flag
void initLoggingFlag()
{
  char *envValue = getenv("LLRT_LOG");
  logEnabled = (envValue != NULL);
}

// Function to get a human-readable timestamp
void getTimestamp(char *timestampBuffer)
{
  struct timeval tv;
  struct tm timeinfo;

  gettimeofday(&tv, NULL);
  localtime_r(&tv.tv_sec, &timeinfo);

  strftime(timestampBuffer, 26, "[%Y-%m-%dT%T", &timeinfo);
  snprintf(timestampBuffer + 20, 6, ".%03ld]", tv.tv_usec / 1000);
}

// Function to print a log message
void printLog(const char *level, const char *format, va_list args)
{

  char timestampBuffer[TIMESTAMP_BUFFER_SIZE];
  getTimestamp(timestampBuffer);
  printf("[%s]%s", level, timestampBuffer);
  vprintf(format, args);
}

// Log Info
void logInfo(const char *format, ...)
{
  if (logEnabled)
  {
    va_list args;
    va_start(args, format);
    printLog("INFO", format, args);
    va_end(args);
    fflush(stdout);
  }
}

// Log Warning
void logWarn(const char *format, ...)
{
  if (logEnabled)
  {
    va_list args;
    va_start(args, format);
    printLog("WARN", format, args);
    va_end(args);
    fflush(stdout);
  }
}

// Log Error
void logError(const char *format, ...)
{
  if (logEnabled)
  {
    va_list args;
    va_start(args, format);
    printLog("ERROR", format, args);
    va_end(args);
    fflush(stdout);
  }
}

static uint32_t calculateSum(uint32_t *array, uint8_t size)
{
  uint32_t sum = 0;
  for (uint8_t i = 0; i < size; i++)
  {
    sum += array[i];
  }
  return sum;
}

static double micro_seconds()
{
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return tv.tv_sec * (1000.0 * 1000.0) + tv.tv_usec;
}

typedef struct
{
  uint32_t srcSize;
  uint32_t dstSize;
  uint32_t id;
  const void *inputBuffer;
  const void *outputBuffer;
} DecompressThreadArgs;

static void *decompressPartial(void *arg)
{
  DecompressThreadArgs *args = (DecompressThreadArgs *)arg;
  size_t srcSize = args->srcSize;
  size_t dstSize = args->dstSize;

  size_t const dSize = ZSTD_decompress((void *)args->outputBuffer, dstSize, args->inputBuffer, srcSize);

  if (ZSTD_isError(dSize))
  {
    printf("%s!\n", ZSTD_getErrorName(dSize));
    return (void *)1;
  }
  return (void *)0;
}

extern char **environ;

static void readData(
    const char *data,
    uint8_t parts,
    uint32_t **inputSizes,
    uint32_t **outputSizes,
    uint8_t **compressedData,
    uint32_t *uncompressedSize)
{
  uint32_t metadataSize = sizeof(uint32_t) * parts;

  // Extract input sizes
  *inputSizes = (uint32_t *)&data[1];

  // Extract output sizes
  *outputSizes = (uint32_t *)&data[1 + metadataSize];

  *uncompressedSize = calculateSum(*outputSizes, parts);

  // Calculate the offset to the compressed data
  uint8_t dataOffset = 1 + (2 * metadataSize);

  *compressedData = (uint8_t *)&data[dataOffset];
}

static void decompress(char **uncompressedData, uint32_t *uncompressedSize, int outputFd)
{

#include "data.c"

  uint8_t parts = data[0];
  uint32_t *inputSizes;
  uint32_t *outputSizes;
  uint32_t inputOffset = 0;
  uint32_t outputOffset = 0;
  char *uncompressed;
  uint8_t *compressedData;

  pthread_t threads[parts];

  if (parts > 1)
  {
    logInfo("Decompressing using %d threads\n", parts);
  }
  else
  {
    logInfo("Decompressing\n");
  }

  readData(data, parts, &inputSizes, &outputSizes, &compressedData, uncompressedSize);

  if (ftruncate(outputFd, *uncompressedSize) == -1)
  {
    err(1, "Failed to set file size");
  }

  uncompressed = mmap(NULL, *uncompressedSize, PROT_READ | PROT_WRITE, MAP_SHARED, outputFd, 0);
  if (uncompressed == MAP_FAILED || !uncompressed)
  {
    err(1, "Memory mapping failed: Unable to map %u bytes. Make sure you have enough memory available", *uncompressedSize);
  }

  DecompressThreadArgs args[parts];
  for (uint32_t i = 0; i < parts; i++)
  {
    args[i].inputBuffer = compressedData + inputOffset;
    args[i].outputBuffer = uncompressed + outputOffset;
    args[i].srcSize = inputSizes[i];
    args[i].dstSize = outputSizes[i];
    args[i].id = i;
    inputOffset += inputSizes[i];
    outputOffset += outputSizes[i];
    if (parts > 1)
    {
      pthread_create(&threads[i], NULL, decompressPartial, (void *)&args[i]);
    }
    else
    {
      if (decompressPartial((void *)&args[i]) > 0)
      {
        err(1, "failed to decompress");
      }
    }
  }

  if (parts > 1)
  {
    for (uint8_t i = 0; i < parts; i++)
    {
      void *result;
      pthread_join(threads[i], &result);
    }
  }

  *uncompressedData = uncompressed;
}

int main(int argc, char *argv[])
{
  initLoggingFlag();

  logInfo("Runtime starting\n");

  char *tmpAppname = strrchr(argv[0], '/');
  char *appname = tmpAppname ? ++tmpAppname : argv[0];

  double t0 = micro_seconds();

  int outputFd = memfd_create_syscall(appname, 0);
  if (outputFd == -1)
  {
    err(1, "Could not create memfd");
  }

  char *uncompressedData;
  uint32_t uncompressedSize;

  decompress(&uncompressedData, &uncompressedSize, outputFd);

  double t1 = micro_seconds();
  logInfo("Runtime starting\n");
  logInfo("Extraction time: %10.4f ms\n", (t1 - t0) / 1000.0);

  if (munmap(uncompressedData, uncompressedSize) == -1)
  {
    err(1, "Failed to unmap memory");
  }

  double t2 = micro_seconds();
  logInfo("Extraction + write time: %10.4f ms\n", (t2 - t0) / 1000.0);

  // char **new_argv = malloc((size_t)(argc + 1) * sizeof *new_argv);
  // for (uint8_t i = 0; i < argc; ++i)
  // {
  //   if (i == 0)
  //   {
  //     size_t length = strlen(appname) + 2;
  //     new_argv[i] = malloc(length);
  //     memcpy(new_argv[i], "/", 1);
  //     memcpy(new_argv[i] + 1, appname, length);
  //     setenv("_", new_argv[i], true);
  //   }
  //   else
  //   {
  //     size_t length = strlen(argv[i]) + 1;
  //     new_argv[i] = malloc(length);
  //     memcpy(new_argv[i], argv[i], length);
  //   }
  // }
  // new_argv[argc] = NULL;

  unsigned long startTime = (unsigned long)(micro_seconds() / 1000.0);

  char startTimeStr[16];
  sprintf(startTimeStr, "%lu", startTime);

  char *memorySizeStr = getenv("AWS_LAMBDA_FUNCTION_MEMORY_SIZE");
  int memorySize = memorySizeStr ? atoi(memorySizeStr) : 128;
  double memoryFactor = 0.8;
  if (memorySize > 512)
  {
    memoryFactor = 0.9;
  }
  if (memorySize > 1024)
  {
    memoryFactor = 0.92;
  }
  if (memorySize > 2048)
  {
    memoryFactor = 0.95;
  }

  char mimallocReserveMemoryMb[16];
  sprintf(mimallocReserveMemoryMb, "%iMiB", (int)(memorySize * memoryFactor));

  char outputFdStr[10];
  sprintf(outputFdStr, "%i", outputFd);

  setenv("_START_TIME", startTimeStr, false);
  setenv("MIMALLOC_RESERVE_OS_MEMORY", mimallocReserveMemoryMb, false);
  setenv("MIMALLOC_LIMIT_OS_ALLOC", "1", false);
  setenv("LLRT_MEM_FD", outputFdStr, false);

  logInfo("Starting app\n");

  fexecve(outputFd, argv, environ);

  logError("Failed to start executable");

  err(1, "fexecve failed");

  return 1;
}