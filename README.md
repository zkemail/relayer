# Relayer

A permissionless relayer server for an email wallet. It receives emails from users with IMAP, generates proofs with ZKP, submits them to an email wallet contract, and replies to the user's email with SMTP. 

## Branch Description
- main:
The main branch only supports halo2-based prover in [halo2-zk-email](https://github.com/zkemail/halo2-zk-email) and smart contracts implemented with an [email-wallet-contracts](https://github.com/zkemail/email-wallet-contracts) template. This proof takes a few minutes on a large server.
It was used to build a demo of ["Contract Wallet Using Emails" at ICBC2023](https://speakerdeck.com/sorasuegami/icbc2023-contract-wallet-using-emails).

- feat/model:
The feat/model branch supports circom-based prover and smart contracts in [zk-email-verify](https://github.com/zkemail/zk-email-verify). The circom prover comes with a Dockerfile and an autoscaling prover. The way this works is that you run the Python coordinator.py on the same small machine that you run the relayer on. Upon receiving an email, it responds and uploads that email to AWS. It also stores the eml file in a new directory that the coordinator is tracking (ideally this is a Redis queue instead). Upon seeing a new validated email added to that directory, the coordinator calls an API endpoint deployed on Modal with that filename that automatically spins up a 64 core instance per proof, so that proving happens in seconds, don't block the main thread, and allow parallel proofs. It takes about 5 seconds to validate [can be [optimized to 1 sec](https://github.com/zkemail/relayer/issues/20)], 25 seconds for witness generation [can be [optimized via C](https://github.com/zkemail/relayer/issues/20) to probably < 5 seconds] and 15 seconds to prove on the server. The final email replies and chain operations take about one second. Total, it takes about 40-45 seconds to prove but could be halved.

When you update the modal endpoint or prover code, you have to redeploy the docker image. This comes from SSH-ing into Aayush's machine where we already did the prover generation code for the final zkey (ideally this is fetched from AWS or IPFS instead, to remove dependency on this machine). It adds those build files to the docker image, currently called `aayushg0/zkemail-modal:modal`, and compiles the latest commits for the relayer and zk-email repositories [you might have to add a force refresh to clear the cache in the dockerfile before the final pulls via adding a `RUN ls` before the last pulls and builds run]. Then, you run the command to update modal to use the new Docker image (`modal deploy --name aayush coordinator.py`, which will force rebuild the API from the latest version of that Dockerfile, as well as overwrite the local env variables with the modal ones and send them to the modal instance).

The integration with the halo2-based prover is under development. Goerli Wallet Address (circom-only): 0x3b3857eaf44804cce00449b7fd40310e6de6496e

## Setup
In a new cloud instance, run:

```
sudo apt update
sudo apt-get install -y pkg-config libssl-dev build-essential nginx certbot python3-certbot-nginx
curl https://sh.rustup.rs -sSf | sh
cargo build --release
ip -4 -o addr show scope global | awk '{print $4}' && ip -6 -o addr show scope global | awk '{print $4}' # Point the DNS to these raw IPs
```

### Enable IMAP in Gmail

Here's how to enable IMAP access and use App Passwords for your Gmail or Google Workspace account:

Enable IMAP:

a. Sign in to your Gmail or Google Workspace account.
b. Click the gear icon in the top-right corner and select "See all settings."
c. Go to the "Forwarding and POP/IMAP" tab.
d. In the "IMAP access" section, select "Enable IMAP."
e. Click "Save Changes."

Create an App Password (recommended):

a. Go to your Google Account settings: https://myaccount.google.com/
b. In the left-hand menu, click "Security."
c. In the "Signing in to Google" section, click on "App Passwords." (Note: This option will only be available if you have 2-Step Verification enabled.)
d. Click on "Select app" and choose "Mail" from the dropdown menu.
e. Click on "Select device" and choose the device you're using or select "Other" to enter a custom name.
f. Click "Generate."
g. Google will generate a 16-character App Password. Make sure to copy and save it securely, as you won't be able to see it again.

Now, when connecting to Gmail or Google Workspace via IMAP, use your email address as the "imap id" (username) and the generated App Password as the "password." If you have not enabled 2-Step Verification and are using "Less secure apps" access, you can use your regular email password instead of the App Password. However, using App Passwords is recommended for enhanced security.

### Enable ports in AWS

If there's an error, make sure your ports are open and traffic is allowed. This will be a massive pain in the \*\*\* so just stay with me while 3 hours of your life dissapate to nonsensical setups. Ensure that your EC2 instance has port 80 and 443 open in the security group to allow incoming traffic. You can check and update your security group settings from the AWS EC2 Management Console.

Step 0 is make sure to always add your IPv4 and IPv6 addresses to A and AAAA records respectively for both @ and www in DNS settings of the domain.

Then, enable inbound traffic. To do so, follow these steps:

0. Log in to the AWS Management Console.
1. Navigate to the EC2 Dashboard.
2. Select "Security Groups" from the left sidebar.
3. Find the security group associated with your EC2 instance and click on its name.
4. Click on the "Inbound rules" tab.
5. For the server, check if there are rules allowing traffic on ports 80 and 443. If not, add the rules by clicking on "Edit inbound rules" and then "Add rule". Choose "HTTP" for port 80 and "HTTPS" for port 443, and set the source to "Anywhere" or "0.0.0.0/0" (IPv4) and "::/0" (IPv6). For IMAP, click on "Add rule" and create new rules for the necessary IMAP ports (143 and 993) with the following settings:

- Type: Custom TCP
- Protocol: TCP
- Port Range: 143 (for IMAP) or 993 (for IMAPS)
- Source: Choose "Anywhere" for both IPv4 and IPv6 (0.0.0.0/0 for IPv4 and ::/0 for IPv6)

0. To rnable IPv6 support for your VPC (Virtual Private Cloud), go to the VPC Dashboard in the AWS Management Console, select your VPC, click on "Actions", and then click on "Edit CIDRs". Add an IPv6 CIDR block.
1. Enable IPv6 support for your subnet. Go to the "Subnets" section in the VPC Dashboard, select the subnet associated with your EC2 instance, click on "Actions", and then click on "Edit IPv6 CIDRs". Add an IPv6 CIDR block.
2. Enable IPv6 support for your EC2 instance. Go to the EC2 Dashboard in the AWS Management Console, select your instance, click on "Actions", and then click on "Manage IP Addresses". Assign an IPv6 address to your instance's network interface.
3. Update your instance's security group to allow inbound IPv6 traffic, if necessary.
4. If needed, configure your operating system to use IPv6. This step depends on the OS you're using. For most Linux distributions, IPv6 is enabled by default.

To enable the security group traffic, run these:

0. Log in to the AWS Management Console and navigate to the EC2 Dashboard: https://console.aws.amazon.com/ec2/
1. In the left-hand menu, click on "Security Groups" under the "Network & Security" section.
2. Locate the security group associated with your instance. You can find the security group in the instance details in the "Instances" section of the EC2 Dashboard.
3. Select the security group and click on the "Inbound rules" tab in the lower panel.
4. Click "Edit inbound rules."
5. Click "Add rule" to create a new rule.
6. Choose the desired rule type (e.g., "HTTP" for port 80, "HTTPS" for port 443, or "Custom TCP" for other ports). In the "Source" field, select "Anywhere-IPv6" to allow traffic from all IPv6 addresses. You can also specify a custom IPv6 range in CIDR notation (e.g., 2001:db8::/32).
7. Click "Save rules" to apply the changes.

Then in AWS EC2 shell, run

```
sudo ufw enable
sudo ufw allow http
sudo ufw allow https
sudo ufw allow ssh
sudo ufw allow 3000
```

Then run the certbot command again.


## Run
```
cargo run --release
```

## Deprecated

### Turn on nginx 

Note that this section is no longer needed, with the addition of the IMAP server direct connection.

````
Configure Nginx: Create a new Nginx configuration file for your application:

```bash
sudo nano /etc/nginx/sites-available/sendeth
````

Paste the following configuration and adjust the domain name and paths accordingly:

```
server {
        listen 80;
        server_name sendeth.org www.sendeth.org;
        return 301 https://$host$request_uri;
}

server {
        listen 443 ssl;
        server_name sendeth.org www.sendeth.org;

        ssl_certificate /etc/letsencrypt/live/sendeth.org/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/sendeth.org/privkey.pem;
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_prefer_server_ciphers on;
    ssl_dhparam /etc/nginx/dhparam.pem;
        ssl_ciphers 'TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GS256-GCM-SHA384:EECDH+AESGCM:EDH+AESGCM'

        location / {
                proxy_pass http://localhost:3000;
                proxy_set_header Host $host;
                proxy_set_header X-Real-IP $remote_addr;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                proxy_set_header X-Forwarded-Proto $scheme;
        }
}
```

We rely on gmail for IMAP, but if you want your own server, you can add this:

```
mail {
    server_name sendeth.com;

    imap_capabilities "IMAP4rev1" "UIDPLUS";

    server {
        listen 143;
        protocol imap;
    }

    server {
        listen 993 ssl;
        protocol imap;
        ssl_certificate /etc/letsencrypt/live/sendeth.com/fullchain.pem;
        ssl_certificate_key /etc/letsencrypt/live/sendeth.com/privkey.pem;
    }
}
```

Save and exit the file. Create a symbolic link to enable the site:

```bash
sudo ln -s /etc/nginx/sites-available/sendeth /etc/nginx/sites-enabled/
```

Test the Nginx configuration and restart Nginx:

```
export YOURDOMAIN=sendeth.org
sudo certbot --nginx -d $YOURDOMAIN -d www.$YOURDOMAIN
```


### Enable TLS/TCP Keepalive (Did not work)

From [here](https://aws.amazon.com/blogs/networking-and-content-delivery/implementing-long-running-tcp-connections-within-vpc-networking/), or else your IMAP connection will drop every 6ish idle minutes.
```
echo -e "net.ipv4.tcp_keepalive_time = 45\nnet.ipv4.tcp_keepalive_intvl = 45\nnet.ipv4.tcp_keepalive_probes = 9" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```
