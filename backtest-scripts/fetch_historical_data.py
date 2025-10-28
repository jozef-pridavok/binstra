#!/usr/bin/env python3

import requests
import json
import time
import hmac
import hashlib
import base64
from datetime import datetime, timedelta
import argparse

try:
    from okx_config import OKX_API_CONFIG
except ImportError:
    print("Warning: okx_config.py not found. Using public API (limited historical data)")
    OKX_API_CONFIG = {
        "api_key": "",
        "api_secret": "",
        "passphrase": "",
        "sandbox": False
    }

def get_okx_headers():
    """Generate OKX API headers with authentication"""
    if not OKX_API_CONFIG["api_key"]:
        return {}
    
    timestamp = str(int(time.time()))
    method = "GET"
    request_path = "/api/v5/market/candles"
    
    # Create signature
    message = timestamp + method + request_path
    signature = base64.b64encode(
        hmac.new(
            OKX_API_CONFIG["api_secret"].encode('utf-8'),
            message.encode('utf-8'),
            hashlib.sha256
        ).digest()
    ).decode('utf-8')
    
    return {
        'OK-ACCESS-KEY': OKX_API_CONFIG["api_key"],
        'OK-ACCESS-SIGN': signature,
        'OK-ACCESS-TIMESTAMP': timestamp,
        'OK-ACCESS-PASSPHRASE': OKX_API_CONFIG["passphrase"],
        'Content-Type': 'application/json'
    }

def fetch_okx_historical_data(symbol, days):
    """Fetch historical price data from OKX API with proper pagination and chunking"""
    
    # Calculate start and end timestamps
    end_time = datetime.now()
    start_time = end_time - timedelta(days=days)
    
    # Convert to milliseconds
    start_ts = int(start_time.timestamp() * 1000)
    end_ts = int(end_time.timestamp() * 1000)
    
    print(f"Fetching data from {start_time} to {end_time}")
    print(f"Total hours needed: {days * 24}")
    
    url = "https://www.okx.com/api/v5/market/candles"
    
    all_data = []
    
    # Break into chunks to handle API limits
    chunk_hours = 100  # Fetch 100 hours at a time
    current_end_ts = end_ts
    
    while current_end_ts > start_ts:
        current_start_ts = max(start_ts, current_end_ts - (chunk_hours * 60 * 60 * 1000))
        
        print(f"Fetching chunk: {datetime.fromtimestamp(current_start_ts/1000)} to {datetime.fromtimestamp(current_end_ts/1000)}")
        
        chunk_data = []
        current_ts = current_end_ts
        
        # Fetch this chunk
        while current_ts > current_start_ts and len(chunk_data) < chunk_hours:
            params = {
                'instId': f'{symbol}-USDT',
                'bar': '1D',  # Daily instead of hourly
                'limit': 100,
                'before': str(current_ts)
            }
            
            try:
                headers = get_okx_headers()
                response = requests.get(url, params=params, headers=headers)
                response.raise_for_status()
                data = response.json()
                
                if data['code'] != '0':
                    print(f"Error from OKX API: {data['msg']}")
                    break
                    
                if not data['data']:
                    print("No more data available from API")
                    break
                    
                candles = data['data']
                added_in_batch = 0
                
                for candle in candles:
                    timestamp = int(candle[0])
                    if timestamp < current_start_ts:
                        break
                        
                    price_data = {
                        'timestamp': datetime.fromtimestamp(timestamp / 1000).isoformat() + 'Z',
                        'prices': {
                            symbol: float(candle[4])  # Close price
                        }
                    }
                    chunk_data.append(price_data)
                    added_in_batch += 1
                    
                if added_in_batch == 0:
                    break
                    
                # Update current timestamp for next iteration
                current_ts = int(candles[-1][0])
                
                # Rate limiting
                time.sleep(0.5)
                
            except requests.RequestException as e:
                print(f"Request error: {e}")
                break
            except Exception as e:
                print(f"Unexpected error: {e}")
                break
        
        print(f"Fetched {len(chunk_data)} candles for this chunk")
        all_data.extend(chunk_data)
        
        # Move to next chunk
        current_end_ts = current_start_ts
        
        # Longer pause between chunks
        if current_end_ts > start_ts:
            print("Pausing 2 seconds between chunks...")
            time.sleep(2)
    
    # Sort by timestamp (oldest first)
    all_data.sort(key=lambda x: x['timestamp'])
    
    print(f"Successfully fetched {len(all_data)} data points")
    if all_data:
        print(f"Date range: {all_data[0]['timestamp']} to {all_data[-1]['timestamp']}")
    
    return all_data

def fetch_fear_greed_data(days):
    """Fetch Fear & Greed index historical data"""
    url = f"https://api.alternative.me/fng/?limit={days}"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        data = response.json()
        
        fear_greed_data = []
        for item in data['data']:
            fear_greed_data.append({
                'timestamp': datetime.fromtimestamp(int(item['timestamp'])).isoformat() + 'Z',
                'value': int(item['value']),
                'classification': item['value_classification']
            })
            
        return fear_greed_data
        
    except requests.RequestException as e:
        print(f"Error fetching Fear & Greed data: {e}")
        return []

def main():
    parser = argparse.ArgumentParser(description='Fetch historical data for backtesting')
    parser.add_argument('--days', type=int, default=180, help='Number of days to fetch (default: 180)')
    parser.add_argument('--symbols', nargs='+', default=['BTC', 'SOL'], help='Crypto symbols to fetch')
    
    args = parser.parse_args()
    
    print(f"Fetching {args.days} days of historical data for {args.symbols}")
    
    # Fetch price data for each symbol
    for symbol in args.symbols:
        print(f"Fetching price data for {symbol}...")
        price_data = fetch_okx_historical_data(symbol, args.days)
        
        if price_data:
            filename = f"../backtest-data/{symbol.lower()}_prices_{args.days}d.json"
            with open(filename, 'w') as f:
                json.dump(price_data, f, indent=2)
            print(f"Saved {len(price_data)} price points to {filename}")
        else:
            print(f"No data fetched for {symbol}")
    
    # Fetch Fear & Greed data
    print("Fetching Fear & Greed index data...")
    fear_greed_data = fetch_fear_greed_data(args.days)
    
    if fear_greed_data:
        filename = f"../backtest-data/fear_greed_{args.days}d.json"
        with open(filename, 'w') as f:
            json.dump(fear_greed_data, f, indent=2)
        print(f"Saved {len(fear_greed_data)} Fear & Greed points to {filename}")
    
    print("Historical data fetch completed!")

if __name__ == "__main__":
    main()