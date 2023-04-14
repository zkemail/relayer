# [WIP] Relayer

A permissionless Rust Axum webserver relayer service that reads email and responds to it. Right now we use the help of some centralized services for the MVP, but soon we will move off of them.

Goerli Wallet Address (circom-only): 0x3b3857eaf44804cce00449b7fd40310e6de6496e

## Setup

In a new cloud instance, run:

```
export YOURDOMAIN=sendeth.org
sudo apt update
sudo apt-get install -y pkg-config libssl-dev build-essential nginx certbot python3-certbot-nginx
curl https://sh.rustup.rs -sSf | sh
cargo build --release
ip -4 -o addr show scope global | awk '{print $4}' && ip -6 -o addr show scope global | awk '{print $4}' # Point the DNS to these raw IPs
```

### Test chain

This verifies that your connection to the chain works and simple txes will send.

```
cargo run --bin chain
```

## Run relayer

```
cargo run --bin relayer
```

### Turn on nginx

````
Configure Nginx: Create a new Nginx configuration file for your application:

```bash
sudo nano /etc/nginx/sites-available/sendeth
````

Paste the following configuration and adjust the domain name and paths accordingly:

```
server {
    listen 80;
    server_name sendeth.com www.sendeth.com;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
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

### Enable ports in AWS

If there's an error, make sure your ports are open and traffic is allowed. This will be a massive pain in the \*\*\* so just stay with me while 3 hours of your life dissapate to nonsensical setups. Ensure that your EC2 instance has port 80 and 443 open in the security group to allow incoming traffic. You can check and update your security group settings from the AWS EC2 Management Console.

Step 0 is make sure to always add your IPv4 and IPv6 addresses to A and AAAA records respectively for both @ and www in DNS settings of the domain.

Then, enable inbound traffic. To do so, follow these steps:

0. Log in to the AWS Management Console.
1. Navigate to the EC2 Dashboard.
2. Select "Security Groups" from the left sidebar.
3. Find the security group associated with your EC2 instance and click on its name.
4. Click on the "Inbound rules" tab.
5. Check if there are rules allowing traffic on ports 80 and 443. If not, add the rules by clicking on "Edit inbound rules" and then "Add rule". Choose "HTTP" for port 80 and "HTTPS" for port 443, and set the source to "Anywhere" or "0.0.0.0/0" (IPv4) and "::/0" (IPv6).

You have to enable IPv4 and IPV6.

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
