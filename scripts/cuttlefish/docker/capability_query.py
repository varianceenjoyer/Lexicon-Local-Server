#!/usr/bin/env python3
"""
Stub capability query script for Cuttlefish VM manager detection.
This provides the minimum capabilities for qemu_cli to work.
"""

import json
import sys

def main():
    # Return capabilities that allow crosvm to work
    capabilities = {
        "qemu_cli": {
            "supported": False,
        },
        "crosvm": {
            "supported": True,
            "virtio_console": True,
            "virtio_fs": True,
            "kvm": True,
        },
        "gem5": {
            "supported": False,
        }
    }

    print(json.dumps(capabilities))
    return 0

if __name__ == "__main__":
    sys.exit(main())
