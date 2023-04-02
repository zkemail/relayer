# [WIP] Relayer

A permissionless Rust Axum webserver relayer service that reads email and responds to it. Right now we use the help of some centralized services for the MVP, but soon we will move off of them.

## Setup

In a new cloud instance, run:

```
export YOURDOMAIN=sendeth.org
sudo apt update
sudo apt install pkg-config libssl-dev
sudo apt-get install libssl-dev
cargo build --release
curl http://169.254.169.254/latest/meta-data/public-ipv4 # This IP will be the one that your DNS record points to
sudo certbot --nginx -d $YOURDOMAIN -d www.$YOURDOMAIN
```

### Enable ports in AWS

If there's an error, make sure your ports are open and traffic is allowed. This will be a massive pain in the \*\*\* so just stay with me while 3 hours of your life dissapate to nonsensical setups. Ensure that your EC2 instance has port 80 and 443 open in the security group to allow incoming traffic. You can check and update your security group settings from the AWS EC2 Management Console.

Step 0 is make sure to always add your IPv4 and IPv6 addresses to A and AAAA records respectively for both @ and www in DNS settings of the domain.

To do this, follow these steps:

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
```

Then run the certbot command again.
