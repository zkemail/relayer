import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess
import os
from dotenv import load_dotenv
stub = modal.Stub()


class DirectoryChangeHandler(FileSystemEventHandler):
    @stub.function(mounts=[
        modal.Mount.from_local_dir("../", remote_path="/root/")],
    )
    def on_created(self, event):
        if not event.is_directory:
            print(f"New file {event.src_path} has been added.")
            file_name = os.path.basename(event.src_path)
            file_name_without_prefix = file_name[file_name.rfind('_') + 1:file_name.rfind('.')]
            subprocess.run(["./src/circom_proofgen.sh", file_name_without_prefix])


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


if __name__ == "__main__":
    load_dotenv()  # Load environment variables from .env file

    path = os.getenv("INCOMING_EML_PATH")
    if path is None:
        print("Error: INCOMING_EML_PATH is not set in the .env file")
        sys.exit(1)

    prove_on_email(path)
