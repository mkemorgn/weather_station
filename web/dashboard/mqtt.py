import paho.mqtt.client as mqtt
import json
from dashboard.db import get_db
from datetime import datetime, timedelta


class MQTTDevice:
    def __init__(self, name, topic, app, socketio=None, broker_host="test.mosquitto.org", broker_port=1883):
        self.name = name
        self.topic = topic
        self.socketio = socketio
        self.latest_payload = None
        self.last_saved = None
        self.client = mqtt.Client(client_id=name)
        self.client.on_connect = self.on_connect
        self.client.on_message = self.on_message
        self.client.connect(broker_host, broker_port, 60)
        self.client.loop_start()
        self.app = app

    def on_connect(self, client, _userdata, _flags, _rc):
        client.subscribe(self.topic)

    def on_message(self, client, userdata, msg):
        try:
            raw = msg.payload.decode()
            parsed = json.loads(raw)
            temp_c = parsed.get("temperature")
            humidity = parsed.get("humidity")

            if temp_c is not None and humidity is not None:
                temp_f = (temp_c * 9/5) + 32
                self.latest_payload = f"{temp_f:.1f}°F / {humidity:.1f}%"

                current_time = datetime.utcnow()
                if self.last_saved is None or (current_time - self.last_saved) >= timedelta(minutes=30):
                    # Insert into DB
                    with self.app.app_context():
                        db = get_db()
                        db.execute(
                            "INSERT INTO sensor_readings (device, topic, temperature, humidity, timestamp) VALUES (?, ?, ?, ?, ?)",
                            (self.name, msg.topic, temp_f, humidity, current_time)
                        )
                        db.commit()
                        self.last_saved = current_time
            else:
                self.latest_payload = raw
        except Exception as e:
            print(f"[{self.name}] Error parsing/saving: {e}")
            self.latest_payload = msg.payload.decode()

        if self.socketio:
            self.socketio.emit("mqtt_data", {
                "device": self.name,
                "topic": msg.topic,
                "payload": self.latest_payload
            })

    def get_data(self):
        return self.latest_payload or "No data yet"

