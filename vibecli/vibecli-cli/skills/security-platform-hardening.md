---
triggers: ["platform hardening", "OS hardening", "CIS benchmark linux", "SSH hardening", "SELinux", "AppArmor", "kernel hardening", "auditd", "file integrity monitoring", "server hardening"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# Platform and OS Hardening

When working with platform hardening:

1. Assess current compliance against CIS benchmarks using automated tools: `sudo cis-cat.sh -b benchmarks/CIS_Ubuntu_Linux_22.04_LTS_Benchmark_v1.0.0.xml --report-format json` for CIS-CAT, or use Lynis for a quick audit: `sudo lynis audit system --profile /etc/lynis/default.prf --report-file lynis-report.dat` which scores the system and provides actionable remediation recommendations.

2. Harden SSH configuration in `/etc/ssh/sshd_config` with security-focused settings: set `PermitRootLogin no`, `PasswordAuthentication no`, `PubkeyAuthentication yes`, `MaxAuthTries 3`, `ClientAliveInterval 300`, `ClientAliveCountMax 2`, `AllowUsers deploy admin`, `Protocol 2`, and `X11Forwarding no`; validate with `sudo sshd -T | grep -E 'permitrootlogin|passwordauth'` and restart with `sudo systemctl restart sshd`.

3. Configure firewall rules using iptables or nftables with a default-deny policy: `sudo iptables -P INPUT DROP && sudo iptables -P FORWARD DROP && sudo iptables -A INPUT -i lo -j ACCEPT && sudo iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT && sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT` then persist with `sudo iptables-save > /etc/iptables/rules.v4`; use `ufw` for simpler management: `sudo ufw default deny incoming && sudo ufw allow 22/tcp && sudo ufw enable`.

4. Enable and configure SELinux for mandatory access control on RHEL/CentOS: verify status with `getenforce` (should return `Enforcing`), set permanently in `/etc/selinux/config` with `SELINUX=enforcing`, use `audit2allow -a -M mypolicy` to create custom policies for legitimate denials, and check context labels with `ls -Z /var/www/html/`.

5. Configure AppArmor profiles on Ubuntu/Debian systems: `sudo aa-status` to list loaded profiles, `sudo aa-enforce /etc/apparmor.d/usr.sbin.nginx` to enforce a profile, create custom profiles with `sudo aa-genprof /usr/bin/myapp` which monitors the application and generates rules interactively, then `sudo aa-logprof` to refine based on logged denials.

6. Apply kernel hardening via sysctl parameters in `/etc/sysctl.d/99-hardening.conf`: set `kernel.randomize_va_space=2` (full ASLR), `net.ipv4.conf.all.rp_filter=1` (reverse path filtering), `net.ipv4.conf.all.accept_redirects=0`, `net.ipv4.conf.all.send_redirects=0`, `kernel.yama.ptrace_scope=1`, `fs.protected_hardlinks=1`, `fs.protected_symlinks=1`; apply with `sudo sysctl --system`.

7. Configure comprehensive audit logging with auditd: install with `sudo apt install auditd`, add rules in `/etc/audit/rules.d/hardening.rules`: `-w /etc/passwd -p wa -k identity`, `-w /etc/shadow -p wa -k identity`, `-w /var/log/ -p wa -k logs`, `-a always,exit -F arch=b64 -S execve -k exec_commands`; load with `sudo augenrules --load` and search with `ausearch -k identity --format csv`.

8. Enable unattended security updates to patch vulnerabilities automatically: on Ubuntu configure `/etc/apt/apt.conf.d/50unattended-upgrades` with `Unattended-Upgrade::Allowed-Origins {"${distro_id}:${distro_codename}-security";}` and `Unattended-Upgrade::Automatic-Reboot "true"`; verify with `sudo unattended-upgrade --dry-run`; on RHEL use `sudo dnf install dnf-automatic && sudo systemctl enable --now dnf-automatic-install.timer`.

9. Minimize the attack surface by disabling unnecessary services: `sudo systemctl list-unit-files --type=service --state=enabled` to inventory, then disable with `sudo systemctl disable --now cups avahi-daemon bluetooth rpcbind`; remove unnecessary packages with `sudo apt autoremove --purge` and verify listening ports with `sudo ss -tlnp` to confirm only required services are exposed.

10. Deploy file integrity monitoring with AIDE or OSSEC: initialize AIDE database with `sudo aideinit && sudo cp /var/lib/aide/aide.db.new /var/lib/aide/aide.db`, schedule daily checks via cron `0 3 * * * /usr/bin/aide --check --config=/etc/aide/aide.conf | mail -s "AIDE Report" security@example.com`, and configure alerts for changes to `/etc/`, `/bin/`, `/sbin/`, and `/usr/bin/`.

11. Harden PAM configuration for strong authentication policies: in `/etc/pam.d/common-password` set `password requisite pam_pwquality.so minlen=14 dcredit=-1 ucredit=-1 ocredit=-1 lcredit=-1 retry=3`, configure account lockout in `/etc/pam.d/common-auth` with `auth required pam_faillock.so deny=5 unlock_time=900 audit`, and enforce session limits in `/etc/security/limits.conf`.

12. Validate hardening posture continuously by scheduling regular compliance scans: run `sudo lynis audit system --cronjob --quiet` weekly via cron, compare Lynis hardening index scores over time (target 80+), export CIS benchmark results to your SIEM, and maintain a hardening baseline document that maps each configuration change to its CIS control ID for audit evidence.
