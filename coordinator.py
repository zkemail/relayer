import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess

class DirectoryChangeHandler(FileSystemEventHandler):
    def on_modified(self, event):
        if event.is_directory:
            print(f"Directory {event.src_path} has been modified.")
            subprocess.run(["./src/circom_proofgen.sh"])

def prove_on_email(path: str):
    event_handler = DirectoryChangeHandler()
    observer = Observer()
    observer.schedule(event_handler, path, recursive=False)
    observer.start()

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()

    observer.join()

stub = modal.Stub()

@stub.webhook(mounts=[
    modal.Mount.from_local_dir("~/", remote_path="/root/")],
)
def run_commands(email_bytes):
    print(email_bytes)
    # Parse email in rust
    print(open("/root/rapidsnark/test.txt").readlines())

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python coordinator.py <directory_path>")
        sys.exit(1)

    path = sys.argv[1]
    prove_on_email(path)

