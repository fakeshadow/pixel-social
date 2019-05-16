import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/SideMenu.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/views/TopicPage.dart';


class TopicsLayout extends StatelessWidget with env {
  final TopicsBloc topicsBloc;

  TopicsLayout({this.topicsBloc});

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
            return SliverToBoxAdapter(
              child: Center(
                child: SizedBox(
                  width: 30,
                  height: 30,
                  child: CircularProgressIndicator(),
                ),
              ),
            );
          }
        });
  }

  pushToTopicPage(context, topic) {
    Navigator.push(context,
        MaterialPageRoute(builder: (context) => TopicPage(topic: topic)));
  }
}