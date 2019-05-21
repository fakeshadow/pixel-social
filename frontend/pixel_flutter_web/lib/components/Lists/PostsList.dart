import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/TopicBlocs.dart';

import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/components/Lists/PostTile.dart';
import 'package:pixel_flutter_web/components/Lists/TopicsList.dart';

class PostsList extends TopicsList {
  final TopicBloc topicBloc;

  PostsList({this.topicBloc});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: topicBloc,
        builder: (context, state) {
          if (state is TopicLoaded) {
            return SliverList(
              delegate: SliverChildBuilderDelegate((context, index) {
                return index >= state.posts.length && !state.hasReachedMax
                    ? BottomLoader()
                    : PostTile(post: state.posts[index]);
              },
                  childCount: state.hasReachedMax
                      ? state.posts.length
                      : state.posts.length + 1),
            );
          } else {
            return Loading();
          }
        });
  }
}
