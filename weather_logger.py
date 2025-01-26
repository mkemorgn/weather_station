import requests
import time
import csv
from bs4 import BeautifulSoup

esp32_url = "http://10.0.0.61"

csv_file = "sensor_data.csv"

def fetch_sensor_data():
    try:
        response = requests.get(esp32_url)
        response.raise_for_status()  # Raise an exception for HTTP errors

        response.encoding = 'utf-8'
        html_content = response.text

        soup = BeautifulSoup(html_content, 'html.parser')
        temperature = soup.find('p', string=lambda x: x and "Temperature" in x).text.split(": ")[1]
        humidity = soup.find('p', string=lambda x: x and "Humidity" in x).text.split(": ")[1]
        pressure = soup.find('p', string=lambda x: x and "Pressure" in x).text.split(": ")[1]

        timestamp = time.strftime("%Y-%m-%d %H:%M:%S")

        return timestamp, temperature, humidity, pressure

    except requests.exceptions.RequestException as e:
        print(f"Error fetching data: {e}")
        return None

def write_to_csv(data):
    try:
        with open(csv_file, mode="a", newline="") as file:
            writer = csv.writer(file)
            writer.writerow(data)
    except Exception as e:
        print(f"Error writing to CSV: {e}")

if __name__ == "__main__":
    with open(csv_file, mode="a", newline="") as file:
        if file.tell() == 0:
            writer = csv.writer(file)
            writer.writerow(["Timestamp", "Temperature", "Humidity", "Pressure"])

    while True:
        sensor_data = fetch_sensor_data()
        if sensor_data:
            print(f"Data fetched: {sensor_data}")
            write_to_csv(sensor_data)
        time.sleep(2)

