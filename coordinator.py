import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess
import os
from dotenv import load_dotenv

# --------- MODAL CLOUD COORDINATOR ------------
image = modal.Image.from_dockerhub(
  "aayushg0/zkemail-modal:modal",
#   setup_dockerfile_commands=[]
).pip_install_from_requirements("requirements.txt")
stub = modal.Stub(image=image)

@stub.function(cpu=4)
@stub.web_endpoint(method="POST")
def test(file_contents: str, nonce: str):
    # Execute the 'pwd' command and capture its output
    result = subprocess.run(['pwd'], capture_output=True, text=True)
    print("pwd: ", result.stdout.strip())

    # Write the file_contents to the file named after the nonce
    file_name = f"wallet_{nonce}.eml"
    with open(file_name, 'w') as file:
        file.write(file_contents)

    # Print the output of the 'proofgen' command
    circom_script_path = "/relayer/src/circom_proofgen.sh"
    result = subprocess.run([circom_script_path, nonce], capture_output=True, text=True)
    print("circom proofgen", result.stdout.strip())
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
