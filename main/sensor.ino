#include <Wire.h>
#include <Adafruit_Sensor.h>
#include <Adafruit_BME280.h>
#include <WiFi.h>
#include "secrets.h"


// Create an instance of the BME280 sensor
Adafruit_BME280 bme;

// Create a Wi-Fi server on port 80
WiFiServer server(80);

void setup() {
  Serial.begin(115200);

  // Configure static IP
  if (!WiFi.config(local_IP, gateway, subnet, primaryDNS, secondaryDNS)) {
    Serial.println("Static IP configuration failed!");
  }

  // Connect to Wi-Fi
  WiFi.begin(ssid, password);
  Serial.println("Connecting to Wi-Fi...");
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }

  // Print connection details
  Serial.println("\nConnected to Wi-Fi!");
  Serial.print("IP Address: ");
  Serial.println(WiFi.localIP());

  // Start the BME280 sensor
  if (!bme.begin(0x76)) { // Adjust to 0x77 if needed
    Serial.println("Could not find a valid BME280 sensor, check wiring!");
    while (1);
  }

  // Start the server
  server.begin();
  Serial.println("Server started!");
}

void loop() {
  // Check if a client has connected
  WiFiClient client = server.accept(); // Use `accept()` instead of `available()`
  if (!client) return;

  Serial.println("New Client connected!");
  String request = client.readStringUntil('\r');
  client.clear(); // Use `clear()` instead of `flush()`

  // Read sensor data
  float temp = bme.readTemperature();
  float humidity = bme.readHumidity();
  float pressure = bme.readPressure() / 100.0F;

  // Create HTML content
  String html = "<!DOCTYPE html><html><body>";
  html += "<h1>BME280 Sensor Data</h1>";
  html += "<p>Temperature: " + String(temp) + " °C</p>";
  html += "<p>Humidity: " + String(humidity) + " %</p>";
  html += "<p>Pressure: " + String(pressure) + " hPa</p>";
  html += "</body></html>";

  // Send HTML response
  client.println("HTTP/1.1 200 OK");
  client.println("Content-type:text/html");
  client.println("Connection: close");
  client.println();
  client.println(html);

  client.stop();
  Serial.println("Client disconnected!");
}
