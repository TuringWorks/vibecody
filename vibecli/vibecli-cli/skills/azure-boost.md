---
triggers: ["Azure Boost", "azure boost", "smartnic", "azure hardware acceleration", "azure storage acceleration", "azure network acceleration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-azure
---

# Azure Boost Infrastructure

When working with Azure Boost:

1. Select Azure Boost-enabled VM sizes (Ebsv5, Dldsv6, Falsv6 families and newer) to automatically benefit from hardware-offloaded networking and storage I/O; these VMs delegate host functions to purpose-built hardware, reducing hypervisor overhead.
2. Leverage SmartNIC offloading for network-intensive workloads where Azure Boost moves virtual switch processing, encryption, and SDN policy enforcement to dedicated FPGA-based network cards, freeing host CPU cycles for your application.
3. Benefit from storage I/O acceleration where Azure Boost offloads remote storage processing to hardware, delivering higher IOPS and lower latency for Premium SSD and Ultra Disk attached volumes compared to software-based storage stacks.
4. Use Accelerated Networking (`--accelerated-networking true` at VM creation) which pairs with Azure Boost's SmartNIC to provide SR-IOV-based networking with sub-100-microsecond latency and up to 200 Gbps throughput on supported SKUs.
5. Understand that Azure Boost reduces the host OS footprint by moving storage, networking, and host management to isolated hardware and firmware; this shrinks the attack surface and eliminates noisy-neighbor effects from host-level processing.
6. Monitor performance gains from Azure Boost using Azure Monitor VM metrics; compare `Disk IOPS Consumed`, `Network Bytes In/Out`, and `CPU Credits Remaining` against non-Boost VMs to quantify the offloading benefit for your workload profile.
7. Deploy latency-sensitive applications (databases, caches, real-time analytics) on Boost-enabled VMs to exploit hardware-accelerated storage paths that bypass the software I/O stack, reducing P99 read latency by up to 45%.
8. Combine Azure Boost with proximity placement groups and availability zones for workloads requiring both low-latency inter-VM communication and high availability; the hardware acceleration complements network proximity for distributed systems.
9. Use Ultra Disks on Boost-enabled VMs for sub-millisecond storage latency with up to 160,000 IOPS per disk; the Boost hardware storage path maximizes throughput to the remote storage tier without CPU bottlenecks.
10. Leverage FPGA-offloaded encryption for data-in-transit and data-at-rest on Boost hardware; TLS termination and disk encryption operations execute on dedicated silicon, maintaining wire-speed performance without consuming VM CPU resources.
11. Plan capacity knowing that Azure Boost VMs allocate more physical CPU cores to the guest because host management functions run on separate hardware; this means the advertised vCPU count delivers closer to bare-metal performance.
12. Migrate existing workloads to Boost-enabled VM SKUs with no application changes required; Azure Boost operates transparently at the infrastructure layer, so applications, drivers, and OS configurations remain identical while benefiting from hardware acceleration.
