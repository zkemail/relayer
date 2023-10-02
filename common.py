import modal

image = (
    modal.Image.from_registry(
        "aayushg0/zkemail-image-updated:modal",
        setup_dockerfile_commands=[
            "RUN apt-get install -y python3 python-is-python3 python3-pip", "RUN cp -r /rapidsnark /root/rapidsnark",
            "RUN cp -r /relayer /root/relayer",
            "RUN cp -r /zk-email-verify /root/zk-email-verify"
        ],
        # force_build=True
    )
    .pip_install_from_requirements("requirements.txt")
)

stub = modal.Stub(image=image)
