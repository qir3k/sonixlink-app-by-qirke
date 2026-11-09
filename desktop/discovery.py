"""
Auto-discovery for SonixLink players on the local network.
Combines:
  1. Ultra-fast non-blocking raw socket probe for /api/info on port 7800
  2. Immediate probe of last-known / candidate IPs (e.g. 192.168.0.17)
  3. UDP beacon listener on port 7801 ("SONIXLINK1 <ip> <port> <name>")
"""

from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
import json
import socket
import threading
import time
from typing import Callable, Dict, List, Optional, Set

BEACON_PORT = 7801
BEACON_PREFIX = "SONIXLINK1"
DEFAULT_PLAYER_PORT = 7800


class DiscoveredPlayer:
    def __init__(self, name: str, host: str, port: int = DEFAULT_PLAYER_PORT, model: str = ""):
        self.name = name
        self.host = host
        self.ip = host
        self.port = port
        self.model = model

    def __eq__(self, other: object) -> bool:
        return isinstance(other, DiscoveredPlayer) and self.host == other.host and self.port == other.port

    def __hash__(self) -> int:
        return hash((self.host, self.port))

    def __repr__(self) -> str:
        return f"DiscoveredPlayer(name='{self.name}', host='{self.host}', port={self.port})"


DeviceBeacon = DiscoveredPlayer


def probe_host_raw(ip: str, port: int = DEFAULT_PLAYER_PORT, timeout: float = 0.35) -> Optional[DiscoveredPlayer]:
    """Fast raw socket HTTP probe for /api/info (aborts unreachable IPs in 350ms)."""
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.settimeout(timeout)
    try:
        s.connect((ip, port))
        req = (
            f"GET /api/info HTTP/1.0\r\n"
            f"Host: {ip}:{port}\r\n"
            f"User-Agent: SonixLink-Probe/1.0\r\n"
            f"Connection: close\r\n\r\n"
        ).encode("ascii")
        s.sendall(req)

        resp_bytes = b""
        s.settimeout(0.5)
        while len(resp_bytes) < 4096:
            chunk = s.recv(1024)
            if not chunk:
                break
            resp_bytes += chunk

        s.close()

        if b"200 OK" in resp_bytes:
            header_end = resp_bytes.find(b"\r\n\r\n")
            if header_end != -1:
                body = resp_bytes[header_end + 4:].decode("utf-8", errors="replace")
                data = json.loads(body)
                name = data.get("name") or data.get("model") or "SonixPlayer"
                model = data.get("model", "")
                return DiscoveredPlayer(name=name, host=ip, port=port, model=model)
    except Exception:
        pass
    finally:
        try:
            s.close()
        except Exception:
            pass
    return None


def get_local_ipv4_subnets() -> List[str]:
    """Return local IPv4 subnets (e.g. ['192.168.0', '192.168.1'])."""
    subnets: Set[str] = set()

    # 1. Connect dummy socket to determine primary outgoing interface
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        parts = ip.split(".")
        if len(parts) == 4:
            subnets.add(".".join(parts[:3]))
    except Exception:
        pass

    # 2. Inspect all local network interface IPs
    try:
        hostname = socket.gethostname()
        for ip in socket.gethostbyname_ex(hostname)[2]:
            if not ip.startswith("127."):
                parts = ip.split(".")
                if len(parts) == 4:
                    subnets.add(".".join(parts[:3]))
    except Exception:
        pass

    # Ensure 192.168.0 is always prioritized for home LAN
    ordered: List[str] = []
    if "192.168.0" in subnets:
        ordered.append("192.168.0")
        subnets.remove("192.168.0")
    else:
        ordered.append("192.168.0")

    ordered.extend(list(subnets))
    return ordered


class DiscoveryService:
    """
    Continuous auto-discovery engine.
    Finds SonixPlayer via UDP beacon and fast concurrent subnet probes.
    """

    def __init__(
        self,
        on_found: Optional[Callable[[DiscoveredPlayer], None]] = None,
        on_device_found: Optional[Callable[[DiscoveredPlayer], None]] = None,
    ):
        self.on_found = on_found or on_device_found
        self.running = False
        self.seen: Set[DiscoveredPlayer] = set()
        self.udp_thread: Optional[threading.Thread] = None
        self.probe_thread: Optional[threading.Thread] = None
        self.udp_sock: Optional[socket.socket] = None

    def start(self) -> None:
        if self.running:
            return
        self.running = True

        # 1. Start UDP beacon listener
        self.udp_thread = threading.Thread(target=self._udp_listen_loop, daemon=True)
        self.udp_thread.start()

        # 2. Start fast active network prober
        self.probe_thread = threading.Thread(target=self._active_probe_loop, daemon=True)
        self.probe_thread.start()

    def stop(self) -> None:
        self.running = False
        if self.udp_sock:
            try:
                self.udp_sock.close()
            except Exception:
                pass
            self.udp_sock = None

    def _report(self, player: DiscoveredPlayer) -> None:
        if player not in self.seen:
            self.seen.add(player)
            if self.on_found:
                self.on_found(player)

    # -------------------------------------------------------------------------
    # 1. UDP Beacon Listener (Port 7801)
    # -------------------------------------------------------------------------

    def _udp_listen_loop(self) -> None:
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
            sock.bind(("", BEACON_PORT))
            sock.settimeout(1.0)
            self.udp_sock = sock
        except Exception:
            return

        while self.running:
            try:
                data, addr = sock.recvfrom(1024)
            except socket.timeout:
                continue
            except Exception:
                break

            try:
                msg = data.decode("utf-8", errors="replace").strip()
                parts = msg.split(" ", 3)
                if len(parts) >= 3 and parts[0] == BEACON_PREFIX:
                    port = int(parts[2])
                    host = addr[0]
                    name = parts[3].strip() if len(parts) > 3 and parts[3].strip() else host
                    self._report(DiscoveredPlayer(name=name, host=host, port=port))
            except Exception:
                pass

        try:
            sock.close()
        except Exception:
            pass

    # -------------------------------------------------------------------------
    # 2. Fast Active Network Prober (Port 7800)
    # -------------------------------------------------------------------------

    def _active_probe_loop(self) -> None:
        while self.running:
            # First: Immediately probe high-priority candidate IPs
            candidates = ["192.168.0.17", "192.168.0.10", "192.168.0.2", "192.168.1.17"]
            for cand in candidates:
                if not self.running:
                    return
                found = probe_host_raw(cand, DEFAULT_PLAYER_PORT, timeout=0.3)
                if found:
                    self._report(found)

            # Second: Scan full local subnets concurrently
            subnets = get_local_ipv4_subnets()
            for subnet in subnets:
                if not self.running:
                    return
                self._scan_subnet_fast(subnet)

            # Pause between background sweeps
            for _ in range(12):
                if not self.running:
                    return
                time.sleep(1.0)

    def _scan_subnet_fast(self, subnet_prefix: str) -> None:
        ips = [f"{subnet_prefix}.{i}" for i in range(1, 255)]

        def worker(ip: str):
            if not self.running:
                return
            found = probe_host_raw(ip, DEFAULT_PLAYER_PORT, timeout=0.35)
            if found:
                self._report(found)

        with ThreadPoolExecutor(max_workers=48) as executor:
            executor.map(worker, ips)


DiscoveryListener = DiscoveryService
