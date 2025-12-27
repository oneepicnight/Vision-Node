"""
Vision Node Wallet - Client-Side Signing Example (Python)

This example demonstrates how to:
1. Generate Ed25519 keypairs
2. Construct canonical transfer messages
3. Sign transfers with Ed25519
4. Submit signed transfers to the Vision node

Dependencies:
    pip install cryptography requests
"""

import os
import struct
import requests
from typing import Optional, Dict, Any
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey
)
from cryptography.hazmat.primitives import serialization

# Configuration
VISION_NODE_URL = 'http://127.0.0.1:7070'


def uint128_to_le(value: int) -> bytes:
    """Convert u128 to 16-byte little-endian bytes"""
    if value < 0 or value >= 2**128:
        raise ValueError(f"Value {value} out of range for u128")
    
    # Pack as two u64 values (little-endian)
    lower = value & 0xFFFFFFFFFFFFFFFF
    upper = (value >> 64) & 0xFFFFFFFFFFFFFFFF
    
    return struct.pack('<QQ', lower, upper)


def uint64_to_le(value: int) -> bytes:
    """Convert u64 to 8-byte little-endian bytes"""
    if value < 0 or value >= 2**64:
        raise ValueError(f"Value {value} out of range for u64")
    
    return struct.pack('<Q', value)


def construct_transfer_message(transfer: Dict[str, Any]) -> bytes:
    """
    Construct canonical message for signing a transfer
    
    Format:
        from (32 bytes raw) +
        to (32 bytes raw) +
        amount (16 bytes LE u128) +
        fee (16 bytes LE u128) +
        nonce (8 bytes LE u64) +
        memo (optional UTF-8)
    
    Args:
        transfer: Dictionary with keys: from, to, amount, fee, nonce, memo
    
    Returns:
        Canonical message bytes
    """
    parts = [
        bytes.fromhex(transfer['from']),              # 32 bytes
        bytes.fromhex(transfer['to']),                # 32 bytes
        uint128_to_le(transfer['amount']),            # 16 bytes
        uint128_to_le(transfer.get('fee', 0)),        # 16 bytes
        uint64_to_le(transfer['nonce']),              # 8 bytes
        transfer.get('memo', '').encode('utf-8')      # optional
    ]
    
    return b''.join(parts)


class VisionWallet:
    """Vision blockchain wallet with Ed25519 signing"""
    
    def __init__(self, private_key: Optional[Ed25519PrivateKey] = None):
        """
        Initialize wallet with optional private key
        
        Args:
            private_key: Ed25519 private key (generates new if None)
        """
        if private_key is None:
            self.private_key = Ed25519PrivateKey.generate()
        else:
            self.private_key = private_key
        
        self.public_key = self.private_key.public_key()
        
        # Get raw bytes for address
        public_key_bytes = self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw
        )
        self.address = public_key_bytes.hex()
    
    @classmethod
    def from_private_key_hex(cls, hex_key: str) -> 'VisionWallet':
        """
        Create wallet from hex-encoded private key
        
        Args:
            hex_key: 64-character hex string (32 bytes)
        
        Returns:
            VisionWallet instance
        """
        private_key_bytes = bytes.fromhex(hex_key)
        private_key = Ed25519PrivateKey.from_private_bytes(private_key_bytes)
        return cls(private_key)
    
    def get_private_key_hex(self) -> str:
        """Get private key as hex string"""
        private_bytes = self.private_key.private_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PrivateFormat.Raw,
            encryption_algorithm=serialization.NoEncryption()
        )
        return private_bytes.hex()
    
    def get_public_key_hex(self) -> str:
        """Get public key as hex string"""
        public_bytes = self.public_key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw
        )
        return public_bytes.hex()
    
    def sign_message(self, message: bytes) -> bytes:
        """
        Sign a message with Ed25519
        
        Args:
            message: Message bytes to sign
        
        Returns:
            64-byte signature
        """
        return self.private_key.sign(message)
    
    def get_balance(self) -> Dict[str, Any]:
        """Query balance from Vision node"""
        url = f'{VISION_NODE_URL}/wallet/{self.address}/balance'
        response = requests.get(url)
        response.raise_for_status()
        return response.json()
    
    def get_nonce(self) -> int:
        """Query current nonce from Vision node"""
        url = f'{VISION_NODE_URL}/wallet/{self.address}/nonce'
        response = requests.get(url)
        response.raise_for_status()
        return response.json()['nonce']
    
    def transfer(
        self,
        to: str,
        amount: int,
        fee: int = 0,
        memo: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Sign and submit a token transfer
        
        Args:
            to: Recipient address (64-char hex)
            amount: Amount to transfer (u128)
            fee: Transfer fee (u128, default 0)
            memo: Optional memo string
        
        Returns:
            Transfer result from node
        
        Raises:
            requests.HTTPError: If transfer fails
        """
        # Get current nonce
        current_nonce = self.get_nonce()
        next_nonce = current_nonce + 1
        
        # Prepare transfer data
        transfer_data = {
            'from': self.address,
            'to': to,
            'amount': amount,
            'fee': fee,
            'nonce': next_nonce,
            'memo': memo or ''
        }
        
        # Construct and sign message
        message = construct_transfer_message(transfer_data)
        signature = self.sign_message(message)
        
        # Prepare request
        request = {
            'from': self.address,
            'to': to,
            'amount': str(amount),
            'fee': str(fee),
            'memo': memo,
            'signature': signature.hex(),
            'nonce': next_nonce,
            'public_key': self.get_public_key_hex()
        }
        
        # Submit to node
        url = f'{VISION_NODE_URL}/wallet/transfer'
        response = requests.post(url, json=request)
        response.raise_for_status()
        
        return response.json()


# ===== USAGE EXAMPLES =====

def example1_generate_wallet():
    """Example 1: Generate a new wallet"""
    print('=== Example 1: Generate Wallet ===')
    
    wallet = VisionWallet()
    
    print(f'Private Key: {wallet.get_private_key_hex()}')
    print(f'Public Key:  {wallet.get_public_key_hex()}')
    print(f'Address:     {wallet.address}')
    print()
    
    return wallet


def example2_query_balance(wallet: VisionWallet):
    """Example 2: Query wallet balance"""
    print('=== Example 2: Query Balance ===')
    
    try:
        balance = wallet.get_balance()
        print(f"Address: {balance['address']}")
        print(f"Balance: {balance['balance']}")
    except requests.HTTPError as e:
        print(f"Error: {e}")
    
    print()


def example3_query_nonce(wallet: VisionWallet):
    """Example 3: Query wallet nonce"""
    print('=== Example 3: Query Nonce ===')
    
    try:
        nonce = wallet.get_nonce()
        print(f'Address: {wallet.address}')
        print(f'Nonce:   {nonce}')
    except requests.HTTPError as e:
        print(f"Error: {e}")
    
    print()


def example4_signed_transfer():
    """Example 4: Execute a signed transfer"""
    print('=== Example 4: Signed Transfer ===')
    
    # Generate sender and recipient wallets
    sender = VisionWallet()
    recipient = VisionWallet()
    
    print(f'Sender:    {sender.address}')
    print(f'Recipient: {recipient.address}')
    
    # NOTE: In production, you would fund the sender address first
    
    try:
        # Get current nonce
        nonce = sender.get_nonce()
        print(f'Current Nonce: {nonce}')
        
        # Execute transfer
        print('\nTransfer Details:')
        print('  Amount: 5000')
        print('  Fee:    50')
        print('  Memo:   Test transfer')
        
        result = sender.transfer(
            to=recipient.address,
            amount=5000,
            fee=50,
            memo='Test transfer'
        )
        
        print('\n✓ Transfer successful!')
        print(f"Result: {result}")
    except requests.HTTPError as e:
        print(f'\n✗ Transfer failed: {e}')
    
    print()


def example5_message_construction():
    """Example 5: Demonstrate message construction"""
    print('=== Example 5: Message Construction ===')
    
    transfer = {
        'from': '0' * 62 + '01',  # 32-byte hex address
        'to': '0' * 62 + '02',    # 32-byte hex address
        'amount': 5000,
        'fee': 50,
        'nonce': 1,
        'memo': 'test'
    }
    
    message = construct_transfer_message(transfer)
    
    print('Transfer:')
    print(f"  from:   {transfer['from']}")
    print(f"  to:     {transfer['to']}")
    print(f"  amount: {transfer['amount']}")
    print(f"  fee:    {transfer['fee']}")
    print(f"  nonce:  {transfer['nonce']}")
    print(f"  memo:   {transfer['memo']}")
    print(f'\nCanonical Message (hex):')
    print(message.hex())
    print(f'\nMessage Length: {len(message)} bytes')
    print('Expected: 32 (from) + 32 (to) + 16 (amount) + 16 (fee) + 8 (nonce) + 4 (memo) = 108')
    print()


def example6_multiple_transfers():
    """Example 6: Execute multiple sequential transfers"""
    print('=== Example 6: Multiple Sequential Transfers ===')
    
    sender = VisionWallet()
    recipient = VisionWallet()
    
    print(f'Sender:    {sender.address}')
    print(f'Recipient: {recipient.address}')
    
    try:
        # Get initial nonce
        nonce = sender.get_nonce()
        print(f'Initial Nonce: {nonce}')
        
        # Execute 3 transfers
        for i in range(1, 4):
            print(f'\nTransfer {i}:')
            
            try:
                result = sender.transfer(
                    to=recipient.address,
                    amount=1000 * i,
                    fee=10,
                    memo=f'Transfer #{i}'
                )
                print(f'  ✓ Success (nonce {nonce + i})')
            except requests.HTTPError as e:
                print(f'  ✗ Failed: {e}')
                break
    except requests.HTTPError as e:
        print(f'Error getting nonce: {e}')
    
    print()


def example7_load_existing_wallet():
    """Example 7: Load wallet from existing private key"""
    print('=== Example 7: Load Existing Wallet ===')
    
    # Generate a wallet
    original = VisionWallet()
    private_key_hex = original.get_private_key_hex()
    
    print('Original Wallet:')
    print(f'  Private Key: {private_key_hex}')
    print(f'  Address:     {original.address}')
    
    # Load wallet from private key
    loaded = VisionWallet.from_private_key_hex(private_key_hex)
    
    print('\nLoaded Wallet:')
    print(f'  Private Key: {loaded.get_private_key_hex()}')
    print(f'  Address:     {loaded.address}')
    
    print(f'\n✓ Addresses match: {original.address == loaded.address}')
    print()


def main():
    """Run all examples"""
    try:
        # Run examples
        wallet = example1_generate_wallet()
        
        # Examples 2-4 require a running Vision node
        # Uncomment to test with real node:
        
        # example2_query_balance(wallet)
        # example3_query_nonce(wallet)
        # example4_signed_transfer()
        
        example5_message_construction()
        
        # example6_multiple_transfers()
        
        example7_load_existing_wallet()
        
        print('✓ All examples completed')
    except Exception as e:
        print(f'Error: {e}')
        import traceback
        traceback.print_exc()


if __name__ == '__main__':
    main()
