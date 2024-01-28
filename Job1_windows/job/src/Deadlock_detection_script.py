import subprocess
import time

start_time = time.time()
path = r".\target\debug\job.exe"
count = 0
def exec_test():
    global path
    global count
    try:
        get_input = subprocess.run([path],
                                    timeout=2,
                                    stdout=subprocess.PIPE)
        print(f"==> {count} ",get_input.stdout)
    except:
        print(f"==> {count}  DeadLock panic!!!")    
    count += 1

def fuzz():
    for _ in range(0,2000):
        exec_test()
    end_time = time.time()
    print(end_time-start_time)

if __name__ == '__main__':
    fuzz()
    