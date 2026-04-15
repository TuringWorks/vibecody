Feature: mDNS/DNS-SD zero-config LAN discovery
  The daemon announces _vibecli._tcp.local. via mDNS so the mobile app
  can discover it on any LAN without IP configuration or special flags.

  Scenario: DNS name encoding produces correct label sequence
    Given the fully-qualified domain name "_vibecli._tcp.local."
    When I encode it as a DNS name
    Then the encoded bytes should start with label length 8
    And the encoded bytes should end with a root null byte

  Scenario: Short input produces valid PTR label for service type
    Given the fully-qualified domain name "_vibecli._tcp.local."
    When I encode it as a DNS name
    Then decoding the labels should yield "_vibecli", "_tcp", "local"

  Scenario: Announce packet has correct DNS header flags
    Given a machine with id "test-machine" on port 7878
    When I build an mDNS announce packet
    Then the packet should be at least 12 bytes long
    And the flags field should equal 0x8400

  Scenario: Announce packet contains expected record count
    Given a machine with id "test-machine" on port 7878
    And the machine has LAN IP "192.168.1.42"
    When I build an mDNS announce packet
    Then the answer count should be at least 3

  Scenario: PTR query detection recognises _vibecli service
    Given a raw DNS PTR query for "_vibecli._tcp.local."
    When I check if it is a VibeCLI PTR query
    Then the result should be true

  Scenario: PTR query detection ignores unrelated service types
    Given a raw DNS PTR query for "_http._tcp.local."
    When I check if it is a VibeCLI PTR query
    Then the result should be false

  Scenario: PTR query detection rejects DNS responses
    Given a raw DNS response packet
    When I check if it is a VibeCLI PTR query
    Then the result should be false

  Scenario: PTR query detection rejects undersized packets
    Given a DNS packet smaller than 12 bytes
    When I check if it is a VibeCLI PTR query
    Then the result should be false

  Scenario: Hostname detection returns a non-empty string
    When I call get_hostname
    Then the hostname should not be empty
    And the hostname should not contain a dot

  Scenario: Local IPv4 address detection returns at least one address
    When I call local_ipv4_addrs
    Then the result should contain at least one address
    And none of the addresses should be the loopback address "127.0.0.1"
