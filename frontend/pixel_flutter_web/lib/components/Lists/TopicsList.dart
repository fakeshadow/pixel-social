import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/Lists/TopicTile.dart';
import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/views/TopicPage.dart';

class TopicsList extends StatelessWidget with env {
  final TopicsBloc topicsBloc;
  final Widget tile;

  TopicsList({this.topicsBloc, this.tile});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: topicsBloc,
        builder: (context, state) {
          if (state is TopicsLoaded) {
            return SliverList(
              delegate: SliverChildBuilderDelegate((context, index) {
                return index >= state.topics.length && !state.hasReachedMax
                    ? BottomLoader()
                    : TopicTile(
                        topic: state.topics[index],
                        onTap: () =>
                            pushToTopicPage(context, state.topics[index]));
              },
                  childCount: state.hasReachedMax
                      ? state.topics.length
                      : state.topics.length + 1),
            );
          } else {
            return Loading();
          }
        });
  }

  Widget Loading() {
    return SliverFillViewport(
        delegate: SliverChildBuilderDelegate((context, index) {
      return Center(
        child: SizedBox(
          width: 30,
          height: 30,
          child: CircularProgressIndicator(),
        ),
      );
    }, childCount: 1));
  }

  pushToTopicPage(context, topic) {
    Navigator.push(context,
        MaterialPageRoute(builder: (context) => TopicPage(topic: topic)));
  }
}
