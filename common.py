import modal

image = (
    modal.Image.from_registry(
        "aayushg0/emailwallet:v0",
        setup_dockerfile_commands=[
            "RUN ls /",
            "RUN cp -r /rapidsnark /root/rapidsnark",
            "RUN cp -r /relayer /root/relayer",
            "RUN cp -r /zk-email-verify /root/zk-email-verify"
        ],
        add_python="3.10",
        force_build=True
    )
    .pip_install_from_requirements("requirements.txt")
)

stub = modal.Stub(image=image)