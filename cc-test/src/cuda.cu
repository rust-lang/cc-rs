#include <cuda.h>

__global__ void kernel() {}

extern "C" void cuda_kernel() { kernel<<<1, 1>>>(); }
