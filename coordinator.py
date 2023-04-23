import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess
import os
from dotenv import load_dotenv

# --------- MODAL CLOUD COORDINATOR ------------
# def filter_condition(file: str):
#     return ".git/" not in file and "target/" not in file

# image = modal.Image.from_dockerfile(
#     "/home/ubuntu/Dockerfile",
#     context_mount=modal.Mount()
#     # .add_local_dir("./relayer", remote_path="/root/relayer", condition=filter_condition)
#     # .add_local_dir("./zk-email-verify/build", remote_path="/root/zk-email-verify/build")
#     # .add_local_dir("./rapidsnark/build", remote_path="/root/rapidsnark/build")
# )
image = modal.Image.from_dockerhub(
  "aayushg0/zkemail-image:modal",
#   setup_dockerfile_commands=[]
)
stub = modal.Stub(image=image)

@stub.function()
def test(file_contents: str):
    return len(file_contents)

# --------- LOCAL COORDINATOR ------------

def is_eml_file(file_name):
    _, file_extension = os.path.splitext(file_name)
    return file_extension.lower() == '.eml'

class DirectoryChangeHandler(FileSystemEventHandler):
    def on_created(self, event):
        if not event.is_directory:
            print(f"New file {event.src_path} has been added.")
            file_name = os.path.basename(event.src_path)
            if (is_eml_file(file_name)):
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
