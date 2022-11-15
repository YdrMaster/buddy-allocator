import subprocess
import sys

def avg(list) -> int:
    a = 0 
    for i in list:
        a += float(i)
    return a / len(list)

# file = subprocess.call(['cargo', 'run', '--example', 'avl'])
if __name__ == "__main__":
    allocate = []
    deallocate = []
    for i in range(0, 1000):
        print(f'times{i}')
        p = subprocess.Popen('cargo run --example avl --release', shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        file = p.stdout.readlines()

        # allocate
        list = file[-2].decode().strip().split()        
        if list[0] == 'allocate':
            allocate.append("".join(filter(lambda s:s in '0123456789.', list[1])))
        else:
            print(list)
 #           with open("output", "w") as FILE:
 #               for line in file:
 #                   FILE.write(str(line))

        list = file[-1].decode().strip().split()        
        if list[0] == 'deallocate':
            deallocate.append("".join(filter(lambda s:s in '0123456789.', list[1])))
        else:
            print(list)

        retval = p.wait()
    import numpy as np
    print(f"allocate\t: {allocate}")
    print('avg: {}'.format(avg(allocate)))
    print("="*30)
    print(f"deallocate\t: {deallocate}")
    print('avg: {}'.format(avg(deallocate)))


