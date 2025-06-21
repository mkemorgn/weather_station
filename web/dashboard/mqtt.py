# your_app/mqtt.py
import paho.mqtt.client as mqtt


class MQTTDevice:
    def __init__(self, name, topic, socketio=None, broker_host="test.mosquitto.org", broker_port=1883):
        self.name = name
        self.topic = topic
        self.socketio = socketio
        self.latest_payload = None
        self.client = mqtt.Client(client_id=name)
        self.client.on_connect = self.on_connect
        self.client.on_message = self.on_message
        self.client.connect(broker_host, broker_port, 60)
        self.client.loop_start()

    def on_connect(self, client, _userdata, _flags, _rc):
        client.subscribe(self.topic)

    def on_message(self, _client, _userdata, msg):
        self.latest_payload = msg.payload.decode()
        if self.socketio:
            self.socketio.emit("mqtt_data", {
                "device": self.name,
                "topic": msg.topic,
                "payload": self.latest_payload
            })

    def get_data(self):
        return self.latest_payload or "No data yet"

