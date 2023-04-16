import modal
import sys
import time
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess
import os
from dotenv import load_dotenv

def filter_condition(file: str):
    return ".git/" not in file and "target/" not in file

image = modal.Image.from_dockerfile(
    "./relayer/Dockerfile",
    context_mount=modal.Mount()
    .add_local_dir("./relayer", remote_path="/root/relayer", condition=filter_condition)
    .add_local_dir("./zk-email-verify/build", remote_path="/root/zk-email-verify/build")
    .add_local_dir("./rapidsnark/build", remote_path="/root/rapidsnark/build")
)
stub = modal.Stub(image=image)

# https://github.com/zkemail/zk-email-verify
# can cp the local ../relayer
# but then need to cargo build
# ../rapidsnark/build

@stub.function()
def test(file_contents: str):
    return len(file_contents)

# @stub.webhook(mounts=[
#     modal.Mount.from_local_dir("../", remote_path="/root/")],
# )
# def prove_on_email(email_bytes):
#     output_file_path = 'email_1.eml'

#     with open(output_file_path, 'wb') as f:
#         f.write(email_bytes)

#     subprocess.run(["./src/circom_proofgen.sh", "1"])

