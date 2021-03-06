import 'package:flutter_web/material.dart';
import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/models/Topic.dart';

class TopicTile extends StatelessWidget with env {
  final Topic topic;
  final Function onTap;

  TopicTile({@required this.topic, this.onTap});

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
                  image: NetworkImage(url + '${topic.avatarUrl}'),
                )),
          ),
          backgroundColor: Colors.white10,
        ),
      ),
      title: InkWell(
        onTap: onTap,
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
      trailing: Icon(Icons.add_comment),
    );
  }
}
