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
// #include <sys/syscall.h>

// #ifdef __x86_64__
// #define MEMFD_CREATE_SYSCALL_ID 319
// #else
// #define MEMFD_CREATE_SYSCALL_ID 279
// #endif

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
  const void *inputBuffer;
  const void *outputBuffer;
  uint32_t id;
} DecompressThreadArgs;

static void *decompressPartial(void *arg)
{
  DecompressThreadArgs *args = (DecompressThreadArgs *)arg;
  size_t srcSize = args->srcSize;
  size_t dstSize = args->dstSize;

  size_t const dSize = ZSTD_decompress((void *)args->outputBuffer, dstSize, args->inputBuffer, srcSize);
  free(args);
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

  fflush(stdout);

  *compressedData = (uint8_t *)&data[dataOffset];
}

static void decompress(char **uncompressedData, uint32_t *uncompressedSize)
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
    printf("Decompressing using %d threads\n", parts);
  }
  else
  {
    printf("Decompressing\n");
  }

  readData(data, parts, &inputSizes, &outputSizes, &compressedData, uncompressedSize);

  uncompressed = (char *)malloc(*uncompressedSize);
  if (!uncompressed)
  {
    err(1, "Memory allocation failed: Unable to allocate %u bytes. Make sure you have enough memory available", *uncompressedSize);
  }

  for (uint32_t i = 0; i < parts; i++)
  {
    DecompressThreadArgs *args = malloc(sizeof(DecompressThreadArgs));
    args->inputBuffer = compressedData + inputOffset;
    args->outputBuffer = uncompressed + outputOffset;
    args->srcSize = inputSizes[i];
    args->dstSize = outputSizes[i];
    args->id = i;
    inputOffset += inputSizes[i];
    outputOffset += outputSizes[i];
    if (parts > 1)
    {
      pthread_create(&threads[i], NULL, decompressPartial, (void *)args);
    }
    else
    {
      if (decompressPartial((void *)args) > 0)
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

// int memfd_create_syscall(const char *name, unsigned flags)
// {

//   return syscall(MEMFD_CREATE_SYSCALL_ID, name, flags);

//   // aarch64 279
//   // x86_64 319
// }

int main(int argc, char *argv[])
{
  printf("Binary launched\n");

  char *tmpAppname = strrchr(argv[0], '/');
  char *appname = tmpAppname ? ++tmpAppname : argv[0];

  double t0 = micro_seconds();

  int outputFd = memfd_create(appname, 0);
  if (outputFd == -1)
  {
    err(1, "Could not create memfd");
  }

  char *uncompressedData;
  uint32_t uncompressedSize;

  decompress(&uncompressedData, &uncompressedSize);

  double t1 = micro_seconds();
  printf("Extraction time: %10.4f ms\n", (t1 - t0) / 1000.0);
  fflush(stdout);

  write(outputFd, uncompressedData, uncompressedSize);
  free(uncompressedData);

  double t2 = micro_seconds();
  printf("Extraction + write time: %10.4f ms\n", (t2 - t0) / 1000.0);
  fflush(stdout);

  char **new_argv = malloc((size_t)(argc + 1) * sizeof *new_argv);
  for (uint8_t i = 0; i < argc; ++i)
  {
    if (i == 0)
    {
      size_t length = strlen(appname) + 2;
      new_argv[i] = malloc(length);
      memcpy(new_argv[i], "/", 1);
      memcpy(new_argv[i] + 1, appname, length);
      setenv("_", new_argv[i], true);
    }
    else
    {
      size_t length = strlen(argv[i]) + 1;
      new_argv[i] = malloc(length);
      memcpy(new_argv[i], argv[i], length);
    }
  }
  new_argv[argc] = NULL;

  unsigned long startTime = (unsigned long)(micro_seconds() / 1000.0);

  char startTimeStr[16];
  sprintf(startTimeStr, "%lu", startTime);

  setenv("_START_TIME", startTimeStr, false);

  printf("Starting app\n");
  fflush(stdout);

  fexecve(outputFd, new_argv, environ);

  err(1, "%s failed", "fexecve");

  return 1;
}