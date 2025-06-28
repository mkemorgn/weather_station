from flask import Blueprint, render_template

from dashboard.db import get_db

bp = Blueprint("history", __name__)

@bp.route('/history', methods=('GET', 'POST'))
def history():
    db = get_db()
    history_data = db.execute( 'SELECT * FROM sensor_readings').fetchall()

    return render_template('history.html', history=history_data)

@bp.route('/history/upstairs', methods=('GET', 'POST'))
def upstairs():

    return render_template('upstairs.html')

@bp.route('/history/living_room', methods=('GET', 'POST'))
def living_room():

    return render_template('living_room.html')

@bp.route('/history/laundry_room', methods=('GET', 'POST'))
def laundry_room():

    return render_template('laundry_room.html')
