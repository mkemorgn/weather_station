# your_app/dashboard.py

from flask import Blueprint, jsonify, render_template, Response, stream_with_context
from .mqtt import MQTTDevice
import time, json

bp = Blueprint("dashboard", __name__)

top = MQTTDevice("upstairs", topic="D8132A722354/sensor_data")
middle = MQTTDevice("middle", topic="C4DEE25BA558/sensor_data")
lower = MQTTDevice("lower", topic="C82E1826BC40/sensor_data")

@bp.route("/")
def index():
    return render_template("index.html")

@bp.route("/data")
def get_data():
    return jsonify({
        "top": top.get_data(),
        "middle": middle.get_data(),
        "lower": lower.get_data()
    })

@bp.route("/stream")
def stream():
    def event_stream():
        while True:
            data = {
                "top": top.get_data(),
                "middle": middle.get_data(),
                "lower": lower.get_data(),
            }
            yield f"data: {json.dumps(data)}\n\n"
            time.sleep(1)

    return Response(stream_with_context(event_stream()), mimetype="text/event-stream")

