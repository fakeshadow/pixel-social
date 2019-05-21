import 'package:flutter_web/material.dart';
import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/models/Post.dart';

class PostTile extends StatelessWidget with env {
  final Post post;

  PostTile({this.post});

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
                  image: NetworkImage(url + '${post.avatarUrl}'),
                )),
          ),
          backgroundColor: Colors.white10,
        ),
      ),
      title: InkWell(
        onTap: () => print('${post.id} pressed'),
        child: Text(
          '${post.postContent}',
          style: TextStyle(
            fontSize: 16.0,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
      subtitle: Text(
        '${post.id}    ${post.username}    ${post.lastReplyTime}',
        style: TextStyle(fontSize: 12.0, fontWeight: FontWeight.w600),
      ),
      trailing: Icon(Icons.add_comment),
    );
  }
}
