CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
CHARSET_MAP = {c: i for i, c in enumerate(CHARSET)}

GENERATORS = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3]

def hrp_expand(hrp):
    return [ord(x) >> 5 for x in hrp] + [0] + [ord(x) & 31 for x in hrp]


def polymod(values):
    chk = 1
    for v in values:
        top = chk >> 25
        chk = ((chk & 0x1ffffff) << 5) ^ v
        for i in range(5):
            if (top >> i) & 1:
                chk ^= GENERATORS[i]
    return chk


def bech32_decode(bech):
    if any(ord(x) < 33 or ord(x) > 126 for x in bech):
        raise ValueError('invalid char range')
    if bech.lower() != bech and bech.upper() != bech:
        raise ValueError('mixed case')
    bech = bech.lower()
    pos = bech.rfind('1')
    if pos < 1 or pos + 7 > len(bech):
        raise ValueError('invalid separator position')
    hrp = bech[:pos]
    data_part = bech[pos+1:]
    data = []
    for ch in data_part:
        if ch not in CHARSET_MAP:
            raise ValueError(f'invalid char {ch}')
        data.append(CHARSET_MAP[ch])
    values = hrp_expand(hrp) + data
    pm = polymod(values)
    ok = (pm == 1)
    return hrp, data, pm, ok


if __name__ == '__main__':
    addrs = [
        'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080',
        'bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gd0p7z9x8p2l6q2h3vx9y'
    ]
    for a in addrs:
        print('address:', a)
        print('bytes:', list(a.encode('ascii')))
        try:
            hrp, data, pm, ok = bech32_decode(a)
            print('hrp=', hrp)
            print('data_len=', len(data))
            print('polymod=', pm)
            print('valid=', ok)
        except Exception as e:
            print('decode error:', e)
        print('---')
