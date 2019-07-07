import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/status.dart' as status;

import 'package:pixel_flutter/env.dart';

class TalkAPI extends env {
  final channel = IOWebSocketChannel.connect(env.WS_URL);

  handleMessage<String>()  {
    channel.stream.listen((message) {
      return message;
    });
  }
}
