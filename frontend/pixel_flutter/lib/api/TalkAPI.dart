import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/io.dart';

import '../env.dart';

TalkAPI sockets = TalkAPI();

class TalkAPI with env {
  static final TalkAPI _sockets = TalkAPI._internal();

  factory TalkAPI() {
    return _sockets;
  }

  TalkAPI._internal();

  IOWebSocketChannel _channel;

  ObserverList<Function> _listeners = ObserverList<Function>();

  connect() {
    _channel = IOWebSocketChannel.connect(WS_URL);
    _channel.stream.listen(_onReceptionOfMessageFromServer);
  }

  close() {
    if (_channel != null) {
      if (_channel.sink != null) {
        _channel.sink.close();
      }
    }
  }

  send(String message) {
    if (_channel != null) {
      if (_channel.sink != null) {
        _channel.sink.add(message);
      }
    }
  }

  addListener(Function callback) {
    _listeners.add(callback);
  }

  removeListener(Function callback) {
    _listeners.remove(callback);
  }

  _onReceptionOfMessageFromServer(message) {
    _listeners.forEach((Function callback) {
      callback(message);
    });
  }
}
