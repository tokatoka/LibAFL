#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int LLVMFuzzerTestOneInput(const uint8_t *Data, size_t Size) {
  if (Size >= 8 && *(uint32_t *)Data == 0xaabbccdd) { }
  char buf[8] = {'a', 'b', 'c', 'd'};
  int sum = 0;
  for(int i = 0; i < 8; i++) {
        sum += buf[0];
        if (i < Size) {
                sum += Data[i];
        }
  }
  fprintf(stderr, "%d\n", sum);
  if (memcmp(Data, buf, 4) == 0) {  }
  return 0;
}
/*
int main() {

  char buf [10] = {0};
  LLVMFuzzerTestOneInput(buf, 10);

}*/
