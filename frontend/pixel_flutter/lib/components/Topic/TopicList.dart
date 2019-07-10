import 'package:flutter/material.dart';
import 'package:pixel_flutter/models/Topic.dart';

import 'package:pixel_flutter/env.dart';

class TopicView extends StatelessWidget with env {
  final Topic topic;

  TopicView({this.topic});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: InkWell(
        onTap: () => print('Avatar pressed'),
        child: CircleAvatar(
          child: Container(
            decoration: BoxDecoration(
                shape: BoxShape.circle,
                image: DecorationImage(
                  fit: BoxFit.fill,
                  image: NetworkImage(url + 'public/' + '${topic.avatarUrl}'),
                )),
          ),
          backgroundColor: Colors.white10,
        ),
      ),
      title: InkWell(
        onTap: () => print('${topic.id} pressed'),
        child: Text(
          '${topic.title}',
          style: TextStyle(
            fontSize: 16.0,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      subtitle: Text(
        '${topic.id}    ${topic.username}    ${topic.lastReplyTime}',
        style: TextStyle(fontSize: 12.0, fontWeight: FontWeight.w600),
      ),
      trailing: Icon(IconData(0x0)),
    );
  }
}
