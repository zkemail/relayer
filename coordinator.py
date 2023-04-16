import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess
import os
from dotenv import load_dotenv
stub = modal.Stub()


@stub.webhook(mounts=[
    modal.Mount.from_local_dir("../", remote_path="/root/")],
)
def prove_on_email(email_bytes):
    output_file_path = 'email_1.eml'

    with open(output_file_path, 'wb') as f:
        f.write(email_bytes)

    subprocess.run(["./src/circom_proofgen.sh", "1"])

